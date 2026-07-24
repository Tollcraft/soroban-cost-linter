use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{exit, Command, Stdio};

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Serialize, Debug)]
struct Span {
    line_start: usize,
    line_end: usize,
    column_start: usize,
    column_end: usize,
}

#[derive(Serialize, Debug)]
struct LintFinding {
    name: String,
    level: String,
    file: String,
    span: Span,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    help: Option<String>,
}

#[derive(Parser, Debug)]
#[command(name = "cargo-cost-lint")]
#[command(about = "CLI wrapper for soroban-cost-linter")]
struct Cli {
    #[arg(long, help = "Path to budget.toml")]
    config: Option<String>,

    #[arg(long, value_enum, default_value_t = OutputFormat::Text, help = "Output format")]
    format: OutputFormat,
}

#[derive(Deserialize, Debug)]
struct BudgetConfig {
    lints: Option<std::collections::HashMap<String, String>>,
}

include!(concat!(env!("OUT_DIR"), "/lint_names.rs"));

fn validate_and_build_flags(config: &BudgetConfig) -> Result<Vec<String>, String> {
    let mut lint_flags = Vec::new();
    if let Some(lints) = &config.lints {
        for (lint, level) in lints {
            if !LINT_NAMES.contains(&lint.as_str()) {
                let valid = LINT_NAMES.join(", ");
                return Err(format!(
                    "Error: Unknown lint name '{}' in budget.toml. Valid lints are: {}",
                    lint, valid
                ));
            }
            let level_flag = match level.as_str() {
                "allow" => "-A",
                "warn" => "-W",
                "deny" => "-D",
                _ => {
                    return Err(format!(
                        "Error: Unknown lint level '{}' for lint '{}'",
                        level, lint
                    ))
                }
            };
            lint_flags.push(format!("{} {}", level_flag, lint));
        }
    }
    Ok(lint_flags)
}

fn resolve_config(config: Option<&str>) -> Option<PathBuf> {
    if let Some(path) = config {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    let budget = PathBuf::from("budget.toml");
    if budget.exists() {
        return Some(budget);
    }
    None
}

fn main() {
    // Skip the first arg if it is "cost-lint" (when invoked as a cargo subcommand)
    let mut args = std::env::args().collect::<Vec<_>>();
    if args.len() > 1 && args[1] == "cost-lint" {
        args.remove(1);
    }

    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => {
            e.print().unwrap();
            exit(1);
        }
    };

    let mut lint_flags = Vec::new();

    if let Some(ref path) = resolve_config(cli.config.as_deref()) {
        eprintln!("Using config: {}", path.display());
        if let Ok(config_str) = fs::read_to_string(path) {
            if let Ok(config) = toml::from_str::<BudgetConfig>(&config_str) {
                match validate_and_build_flags(&config) {
                    Ok(flags) => lint_flags = flags,
                    Err(e) => {
                        eprintln!("{}", e);
                        exit(1);
                    }
                }
            } else {
                eprintln!("Warning: Failed to parse {}", path.display());
            }
        }
    } else {
        eprintln!("Warning: budget.toml not found, using default lint levels.");
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("dylint");
    cmd.arg("--lib");
    cmd.arg("soroban_cost_lints");

    if !lint_flags.is_empty() {
        let mut rustflags = std::env::var("DYLINT_RUSTFLAGS").unwrap_or_default();
        for flag in lint_flags {
            if !rustflags.is_empty() {
                rustflags.push(' ');
            }
            rustflags.push_str(&flag);
        }
        cmd.env("DYLINT_RUSTFLAGS", rustflags);
    }

    if cli.format == OutputFormat::Json {
        cmd.arg("--");
        cmd.arg("--message-format=json");
        cmd.stdout(Stdio::piped());
    }

    let mut child = cmd
        .spawn()
        .expect("Failed to execute cargo dylint. Is cargo-dylint installed?");

    if cli.format == OutputFormat::Json {
        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let reader = BufReader::new(stdout);
        let mut highest_exit_code = 0;

        for line_str in reader.lines().map_while(Result::ok) {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line_str) {
                if msg.get("reason").and_then(|r| r.as_str()) == Some("compiler-message") {
                    if let Some(message) = msg.get("message") {
                        if let Some(code) = message.get("code") {
                            if let Some(lint_name) = code.get("code").and_then(|c| c.as_str()) {
                                if LINT_NAMES.contains(&lint_name) {
                                    let level = message
                                        .get("level")
                                        .and_then(|l| l.as_str())
                                        .unwrap_or("unknown");
                                    if level == "error" || level == "deny" {
                                        highest_exit_code = 1;
                                    }

                                    let msg_text = message
                                        .get("message")
                                        .and_then(|m| m.as_str())
                                        .unwrap_or("");
                                    let mut file = String::new();
                                    let mut span_obj = Span {
                                        line_start: 0,
                                        line_end: 0,
                                        column_start: 0,
                                        column_end: 0,
                                    };

                                    if let Some(spans) =
                                        message.get("spans").and_then(|s| s.as_array())
                                    {
                                        for s in spans {
                                            if s.get("is_primary")
                                                .and_then(|p| p.as_bool())
                                                .unwrap_or(false)
                                            {
                                                file = s
                                                    .get("file_name")
                                                    .and_then(|f| f.as_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                span_obj.line_start = s
                                                    .get("line_start")
                                                    .and_then(|l| l.as_u64())
                                                    .unwrap_or(0)
                                                    as usize;
                                                span_obj.line_end = s
                                                    .get("line_end")
                                                    .and_then(|l| l.as_u64())
                                                    .unwrap_or(0)
                                                    as usize;
                                                span_obj.column_start = s
                                                    .get("column_start")
                                                    .and_then(|c| c.as_u64())
                                                    .unwrap_or(0)
                                                    as usize;
                                                span_obj.column_end = s
                                                    .get("column_end")
                                                    .and_then(|c| c.as_u64())
                                                    .unwrap_or(0)
                                                    as usize;
                                                break;
                                            }
                                        }
                                    }

                                    let mut help_text = None;
                                    if let Some(children) =
                                        message.get("children").and_then(|c| c.as_array())
                                    {
                                        for child_item in children {
                                            if child_item.get("level").and_then(|l| l.as_str())
                                                == Some("help")
                                            {
                                                help_text = child_item
                                                    .get("message")
                                                    .and_then(|m| m.as_str())
                                                    .map(|s| s.to_string());
                                                break;
                                            }
                                        }
                                    }

                                    let finding = LintFinding {
                                        name: lint_name.to_string(),
                                        level: level.to_string(),
                                        file,
                                        span: span_obj,
                                        message: msg_text.to_string(),
                                        help: help_text,
                                    };

                                    if let Ok(json_str) = serde_json::to_string(&finding) {
                                        println!("{}", json_str);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let status = child.wait().expect("Failed to wait on cargo dylint");
        if !status.success() {
            exit(status.code().unwrap_or(1));
        } else if highest_exit_code != 0 {
            exit(highest_exit_code);
        }
    } else {
        let status = child.wait().expect("Failed to wait on cargo dylint");
        if !status.success() {
            exit(status.code().unwrap_or(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config() {
        let mut lints = std::collections::HashMap::new();
        lints.insert("soroban_storage_in_loop".to_string(), "deny".to_string());
        let config = BudgetConfig { lints: Some(lints) };
        let result = validate_and_build_flags(&config);
        assert!(result.is_ok());
        let flags = result.unwrap();
        assert_eq!(flags, vec!["-D soroban_storage_in_loop"]);
    }

    #[test]
    fn test_unknown_lint_name() {
        let mut lints = std::collections::HashMap::new();
        lints.insert("soroban_storage_in_loops".to_string(), "deny".to_string());
        let config = BudgetConfig { lints: Some(lints) };
        let result = validate_and_build_flags(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown lint name"));
    }

    #[test]
    fn test_unknown_lint_level() {
        let mut lints = std::collections::HashMap::new();
        lints.insert("soroban_storage_in_loop".to_string(), "denys".to_string());
        let config = BudgetConfig { lints: Some(lints) };
        let result = validate_and_build_flags(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown lint level"));
    }
}
