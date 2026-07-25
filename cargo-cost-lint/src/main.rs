use clap::{Parser, ValueEnum};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<String>,
}

#[derive(Parser, Debug)]
#[command(name = "cargo-cost-lint")]
#[command(about = "CLI wrapper for soroban-cost-linter")]
struct Cli {
    #[arg(long, help = "Path to budget.toml")]
    config: Option<String>,

    #[arg(long, value_enum, default_value_t = OutputFormat::Text, help = "Output format")]
    format: OutputFormat,

    #[arg(long, help = "Automatically apply fixable lint suggestions")]
    fix: bool,
}

#[derive(Deserialize, Debug)]
struct BudgetConfig {
    // Reserved for budget.toml lint-level overrides; the validation logic
    // that reads this is a pre-existing stub, unrelated to .lintignore.
    #[allow(dead_code)]
    lints: Option<std::collections::HashMap<String, String>>,
}

include!(concat!(env!("OUT_DIR"), "/lint_names.rs"));

/// Walks `root`, respecting `.gitignore` and `.lintignore`, and returns the
/// canonicalized set of files that are allowed to be linted (i.e. not
/// excluded by either ignore file).
fn allowed_files(root: &Path) -> HashSet<PathBuf> {
    WalkBuilder::new(root)
        .git_ignore(true)
        .add_custom_ignore_filename(".lintignore")
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|entry| entry.path().canonicalize().ok())
        .collect()
}

/// Decides whether a lint finding for `file` should be reported, given the
/// set of files allowed by `.lintignore`/`.gitignore`.
///
/// If `file` is empty or can't be resolved to a canonical path, the finding
/// is kept: a filtering bug must never silently swallow a real diagnostic.
fn is_reportable(file: &str, allowed: &HashSet<PathBuf>) -> bool {
    if file.is_empty() {
        return true;
    }
    match Path::new(file).canonicalize() {
        Ok(canon) => allowed.contains(&canon),
        Err(_) => true,
    }
}

fn main() {
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

    let allowed = allowed_files(Path::new("."));

    let lint_flags: Vec<String> = Vec::new();
    if let Some(config_path) = &cli.config {
        if Path::new(config_path).exists() {
            if let Ok(config_str) = fs::read_to_string(config_path) {
                if let Ok(config) = toml::from_str::<BudgetConfig>(&config_str) {
                    // ... validate (existing code)
                    let _ = config;
                }
            }
        }
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

    // Always ask dylint for JSON diagnostics internally, regardless of the
    // user-facing --format, so .lintignore filtering applies to both
    // text and json output. Text mode renders `message.rendered` for each
    // surviving finding, which matches cargo dylint's normal human output.
    cmd.arg("--");
    cmd.arg("--message-format=json");
    cmd.stdout(Stdio::piped());

    let mut child = cmd
        .spawn()
        .expect("Failed to execute cargo dylint. Is cargo-dylint installed?");

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let reader = BufReader::new(stdout);
    let mut highest_exit_code = 0;

    let mut findings: Vec<LintFinding> = Vec::new();

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

                                if let Some(spans) = message.get("spans").and_then(|s| s.as_array())
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

                                if !is_reportable(&file, &allowed) {
                                    continue;
                                }

                                if level == "error" || level == "deny" {
                                    highest_exit_code = 1;
                                }

                                let mut help_text = None;
                                let mut suggestion = None;
                                if let Some(children) =
                                    message.get("children").and_then(|c| c.as_array())
                                {
                                    for child_item in children {
                                        if child_item.get("level").and_then(|l| l.as_str())
                                            == Some("help")
                                        {
                                            let child_msg = child_item
                                                .get("message")
                                                .and_then(|m| m.as_str())
                                                .map(|s| s.to_string());
                                            help_text = child_msg.clone();
                                            if cli.fix {
                                                suggestion =
                                                    extract_suggestion(&child_msg, lint_name);
                                            }
                                            break;
                                        }
                                    }
                                }

                                let finding = LintFinding {
                                    name: lint_name.to_string(),
                                    level: level.to_string(),
                                    file: file.clone(),
                                    span: span_obj,
                                    message: msg_text.to_string(),
                                    help: help_text,
                                    suggestion,
                                };

                                findings.push(finding);

                                if cli.format == OutputFormat::Json {
                                    let finding_json = findings.last().unwrap();
                                    if let Ok(json_str) = serde_json::to_string(finding_json) {
                                        println!("{}", json_str);
                                    }
                                } else {
                                    let rendered = message
                                        .get("rendered")
                                        .and_then(|r| r.as_str())
                                        .unwrap_or(msg_text);
                                    print!("{}", rendered);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if cli.fix {
        apply_fixes(&findings);
    }

    let status = child.wait().expect("Failed to wait on cargo dylint");
    if !status.success() {
        exit(status.code().unwrap_or(1));
    } else if highest_exit_code != 0 {
        exit(highest_exit_code);
    }
}

fn extract_suggestion(help: &Option<String>, lint_name: &str) -> Option<String> {
    let help_text = help.as_ref()?;
    match lint_name {
        "symbol_new_for_short_literal" => {
            if let Some(start) = help_text.find("symbol_short!(") {
                let end = help_text[start..].find(')')? + start + 1;
                Some(help_text[start..end].to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn apply_fixes(findings: &[LintFinding]) {
    let mut file_edits: std::collections::HashMap<String, Vec<(usize, String, String)>> =
        std::collections::HashMap::new();

    for finding in findings {
        if let Some(ref suggestion) = finding.suggestion {
            let file = finding.file.clone();
            if file.is_empty() {
                continue;
            }
            let line_idx = finding.span.line_start;
            file_edits
                .entry(file)
                .or_default()
                .push((line_idx, finding.message.clone(), suggestion.clone()));
        }
    }

    for (file_path, edits) in &file_edits {
        if let Ok(content) = fs::read_to_string(file_path) {
            let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
            for (line_idx, _message, suggestion) in edits {
                if *line_idx > 0 && *line_idx <= lines.len() {
                    let line = &mut lines[*line_idx - 1];
                    if let Some(start) = line.find("Symbol::new") {
                        if let Some(end) = line[start..].find(')') {
                            let replace_end = start + end + 1;
                            line.replace_range(start..replace_end, suggestion);
                        }
                    }
                }
            }
            let new_content = lines.join("\n");
            if let Err(e) = fs::write(file_path, new_content) {
                eprintln!("Failed to write {}: {}", file_path, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    fn write_file(path: &Path, contents: &str) {
        let mut f = File::create(path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
    }

    #[test]
    fn allowed_files_excludes_lintignore_patterns() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        write_file(&root.join(".lintignore"), "ignored.rs\n");
        write_file(&root.join("keep.rs"), "fn main() {}");
        write_file(&root.join("ignored.rs"), "fn main() {}");

        let allowed = allowed_files(root);

        let keep_canon = root.join("keep.rs").canonicalize().unwrap();
        let ignored_canon = root.join("ignored.rs").canonicalize().unwrap();

        assert!(allowed.contains(&keep_canon));
        assert!(!allowed.contains(&ignored_canon));
    }

    #[test]
    fn allowed_files_respects_gitignore_too() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // ignore's git_ignore(true) only honors .gitignore inside an actual
        // git working tree, so the fixture needs a .git directory present.
        std::fs::create_dir(root.join(".git")).unwrap();
        write_file(&root.join(".gitignore"), "skip.rs\n");
        write_file(&root.join("skip.rs"), "fn main() {}");
        write_file(&root.join("keep.rs"), "fn main() {}");

        let allowed = allowed_files(root);

        assert!(allowed.contains(&root.join("keep.rs").canonicalize().unwrap()));
        assert!(!allowed.contains(&root.join("skip.rs").canonicalize().unwrap()));
    }

    #[test]
    fn is_reportable_keeps_files_in_allowed_set() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_file(&root.join("a.rs"), "fn main() {}");

        let mut allowed = HashSet::new();
        allowed.insert(root.join("a.rs").canonicalize().unwrap());

        let path_str = root.join("a.rs").to_string_lossy().to_string();
        assert!(is_reportable(&path_str, &allowed));
    }

    #[test]
    fn is_reportable_filters_files_not_in_allowed_set() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_file(&root.join("a.rs"), "fn main() {}");
        write_file(&root.join("b.rs"), "fn main() {}");

        let mut allowed = HashSet::new();
        allowed.insert(root.join("a.rs").canonicalize().unwrap());

        let path_str = root.join("b.rs").to_string_lossy().to_string();
        assert!(!is_reportable(&path_str, &allowed));
    }

    #[test]
    fn is_reportable_defaults_to_keeping_unresolvable_paths() {
        let allowed: HashSet<PathBuf> = HashSet::new();
        assert!(is_reportable("this/path/does/not/exist.rs", &allowed));
    }

    #[test]
    fn is_reportable_keeps_empty_file_field() {
        let allowed: HashSet<PathBuf> = HashSet::new();
        assert!(is_reportable("", &allowed));
    }
}
