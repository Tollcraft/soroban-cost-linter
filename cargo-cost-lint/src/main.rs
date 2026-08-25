#[allow(dead_code)]
mod budget_config;
mod config;
mod error;
#[allow(dead_code)]
mod lint_name_set;
mod output_formatters;

use clap::{ArgGroup, Parser};
use output_formatters::{LintFinding, OutputFormat, Span};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, exit};

#[derive(Parser, Debug)]
#[command(name = "cargo-cost-lint")]
#[command(version)]
#[command(about = "CLI wrapper for soroban-cost-linter")]
#[command(group(
    ArgGroup::new("verbosity")
        .args(["quiet", "verbose"])
        .multiple(false)
))]
struct Cli {
    #[arg(long, help = "Path to budget.toml")]
    config: Option<String>,

    #[arg(long, help = "Emit the lint inventory and exit")]
    list_lints: bool,

    #[arg(
        long,
        value_name = "LINT",
        help = "Print the full documentation page for a lint and exit"
    )]
    explain: Option<String>,

    #[arg(long, value_enum, default_value_t = OutputFormat::Text, help = "Output format")]
    format: OutputFormat,

    #[arg(long, help = "Suppress informational and warning output")]
    quiet: bool,

    #[arg(
        long,
        help = "Show diagnostic detail: config path, lint flags, spawned command"
    )]
    verbose: bool,
}

#[derive(Deserialize, Debug)]
struct BudgetConfig {
    lints: Option<std::collections::HashMap<String, String>>,
}

include!(concat!(env!("OUT_DIR"), "/lint_names.rs"));
include!(concat!(env!("OUT_DIR"), "/lint_metadata.rs"));
include!(concat!(env!("OUT_DIR"), "/lint_info.rs"));
include!(concat!(env!("OUT_DIR"), "/lint_explanations.rs"));

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
                    ));
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

/// Loads `path` as a validated `BudgetConfig` and formats its `[lints]`
/// entries into `-A`/`-W`/`-D` flags for `DYLINT_RUSTFLAGS`. Validation
/// (unknown lint names, invalid levels) is handled by
/// `BudgetConfig::from_file_validated`, the single canonical config parser.
// Not currently reached from `main()`, which still uses the inline
// `validate_and_build_flags` path. `cargo-cost-lint` now carries two config
// generations -- this one via `BudgetConfig::from_file_validated` (validates
// lint names and levels) and the newer `config::Config::from_file_or_default`
// (fallback defaults, no name validation). Both are tested; picking which one
// ships is a behavioural decision for a maintainer, so this change leaves
// `main()` as it found it rather than choosing silently.
// Kept: scaffolding for future feature implementations
#[allow(dead_code)]
fn parse_budget_config(path: &str) -> Result<Vec<String>, String> {
    let config = config::BudgetConfig::from_file_validated(Path::new(path), LINT_NAMES)?;

    let mut lint_flags = Vec::new();
    if let Some(lints) = config.lints {
        for (lint, level) in lints {
            let flag = match level.as_str() {
                "allow" => "-A",
                "warn" => "-W",
                "deny" => "-D",
                _ => unreachable!("level already validated by BudgetConfig::from_file_validated"),
            };
            lint_flags.push(format!("{} {}", flag, lint));
        }
    }

    Ok(lint_flags)
}

/// Lenient wrapper around [`parse_budget_config`] that uses safe defaults
/// when the `budget.toml` file cannot be read or parsed, while still
/// propagating validation errors (unknown lint name or level) so that
/// real user mistakes remain loud.
///
/// This implements the "safe defaults" semantics requested by issue
/// #191: a missing, unreadable, empty, or syntactically invalid file
/// produces a stderr warning and an empty flag set, instead of
/// aborting the lint run. Validation errors — which indicate the user
/// actually wrote something wrong — are returned unchanged so the
/// strict behaviour already covered by [`parse_budget_config`] and
/// its existing tests is preserved.
// Kept: scaffolding for future feature implementations
#[allow(dead_code)]
fn try_parse_budget_config(path: &str) -> Result<Vec<String>, String> {
    match parse_budget_config(path) {
        Ok(flags) => Ok(flags),
        Err(e)
            if e.starts_with("Error: Failed to read")
                || e.starts_with("Error: Failed to parse") =>
        {
            eprintln!("warning: {e}\n         continuing with safe defaults (no lint overrides).");
            Ok(Vec::new())
        }
        Err(e) => Err(e),
    }
}

#[allow(clippy::collapsible_if)]
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

    if cli.list_lints {
        if cli.format == OutputFormat::Json {
            println!("{}", serde_json::to_string_pretty(&LINT_INVENTORY).unwrap());
        } else {
            println!("Lint inventory (version {}):", LINT_INVENTORY.version);
            for lint in LINT_INVENTORY.lints {
                println!(
                    "{} | {} | {} | {}",
                    lint.name, lint.default_level, lint.category, lint.documentation_url
                );
            }
        }
        return;
    }

    if let Some(lint_name) = &cli.explain {
        print_explanation(lint_name);
        return;
    }

    let quiet = cli.quiet;
    let verbose = cli.verbose;
    let mut lint_flags = Vec::new();
    let mut resolved_config_path: Option<PathBuf> = None;

    if let Some(ref path) = resolve_config(cli.config.as_deref()) {
        resolved_config_path = Some(path.clone());
        if !quiet {
            eprintln!("Using config: {}", path.display());
        }
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
                if !quiet {
                    eprintln!("Warning: Failed to parse {}", path.display());
                }
            }
        }
    } else {
        if !quiet {
            eprintln!("Warning: budget.toml not found, using default lint levels.");
        }
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("dylint");
    cmd.arg("--lib");
    cmd.arg("soroban_cost_lints");

    let mut rustflags_value = String::new();
    if !lint_flags.is_empty() {
        rustflags_value = std::env::var("DYLINT_RUSTFLAGS").unwrap_or_default();
        for flag in lint_flags {
            if !rustflags_value.is_empty() {
                rustflags_value.push(' ');
            }
            rustflags_value.push_str(&flag);
        }
        cmd.env("DYLINT_RUSTFLAGS", &rustflags_value);
    }

    if verbose {
        if let Some(ref p) = resolved_config_path {
            eprintln!("[verbose] config: {}", p.display());
        } else {
            eprintln!("[verbose] config: (none — using default lint levels)");
        }
        if !rustflags_value.is_empty() {
            eprintln!("[verbose] DYLINT_RUSTFLAGS: {}", rustflags_value);
        } else {
            eprintln!("[verbose] DYLINT_RUSTFLAGS: (empty)");
        }
        eprintln!("[verbose] command: {:?}", cmd);
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
                                        suggestion: None,
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

/// Prints the explanation for a lint, or errors with valid lint names if not found.
fn print_explanation(lint_name: &str) {
    let normalized = lint_name.to_lowercase();

    let explanation = LINT_EXPLANATIONS.iter().find(|e| e.name == normalized);

    match explanation {
        Some(entry) => {
            // Clean up the markdown for terminal display
            let cleaned = clean_markdown_for_terminal(entry.markdown);
            println!("{}", cleaned);
        }
        None => {
            eprintln!("Error: unknown lint '{}'.\n\nValid lints:\n", lint_name);
            for info in LINT_INFO {
                eprintln!("  {} — {}", info.name, info.description);
            }
            exit(1);
        }
    }
}

/// Strips GitBook-specific hint syntax and lightens the markdown for plain
/// terminal output. The result is still markdown-ish but without block-level
/// tags that only render on a documentation site.
fn clean_markdown_for_terminal(markdown: &str) -> String {
    let mut result = String::new();
    let mut in_code_block = false;

    for line in markdown.lines() {
        let trimmed = line.trim();

        // Track code-fence boundaries so we don't strip inside them
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        if in_code_block {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Strip GitBook hint tags and their content delimiters
        if trimmed.starts_with("{% hint")
            || trimmed.starts_with("{% endhint")
            || trimmed.ends_with("%}") && !line.starts_with("    ")
        {
            // Skip hint markers entirely
            continue;
        }

        // Replace bold markers with plain text for terminal
        let cleaned = line.replace("**", "");

        result.push_str(&cleaned);
        result.push('\n');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn lint_metadata_matches_registered_lints() {
        let registered_names = LINT_NAMES
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        let inventory_names = LINT_INVENTORY
            .lints
            .iter()
            .map(|lint| lint.name)
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(registered_names, inventory_names);
        assert_eq!(LINT_INVENTORY.version, "1.0");
    }

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

    #[test]
    fn every_registered_lint_has_explanation_text() {
        for info in LINT_INFO {
            let explanation = LINT_EXPLANATIONS.iter().find(|e| e.name == info.name);
            assert!(
                explanation.is_some(),
                "lint '{}' is registered but has no explanation text. \
                 Add a documentation page at docs/lints/{}.md",
                info.name,
                info.name
            );
            if let Some(entry) = explanation {
                assert!(
                    !entry.markdown.is_empty(),
                    "lint '{}' has an empty explanation text",
                    info.name
                );
            }
        }
    }

    /// Verifies that document has at least the structure of valid markdown
    /// by checking it contains the key sections.
    #[test]
    fn every_explanation_has_key_sections() {
        for entry in LINT_EXPLANATIONS {
            assert!(
                entry.markdown.contains("## What it does"),
                "lint '{}' explanation is missing 'What it does' section",
                entry.name
            );
            assert!(
                entry.markdown.contains("## Why is this bad")
                    || entry.markdown.contains("## Why is this bad?"),
                "lint '{}' explanation is missing 'Why is this bad' section",
                entry.name
            );
        }
    }

    #[test]
    fn print_explanation_known_lint_succeeds() {
        // Test that the first registered lint's explanation resolves
        // correctly and produces terminal-clean output.
        let first = LINT_INFO.first().expect("at least one lint registered");
        let explanation = LINT_EXPLANATIONS.iter().find(|e| e.name == first.name);
        assert!(
            explanation.is_some(),
            "lint '{}' should have explanation",
            first.name
        );
        let entry = explanation.unwrap();
        assert!(
            !entry.markdown.is_empty(),
            "explanation should not be empty"
        );
        // The cleaned output should not contain GitBook hint tags
        let cleaned = clean_markdown_for_terminal(entry.markdown);
        assert!(
            !cleaned.contains("{% hint"),
            "cleaned output should not contain GitBook hint tags"
        );
    }

    #[test]
    fn clean_markdown_removes_hint_tags() {
        let input = "{% hint style=\"danger\" %}\nSome content\n{% endhint %}";
        let cleaned = clean_markdown_for_terminal(input);
        assert!(
            !cleaned.contains("{% hint"),
            "hint open tag should be removed"
        );
        assert!(
            !cleaned.contains("{% endhint"),
            "endhint tag should be removed"
        );
        // Content between hint tags should remain
        assert!(
            cleaned.contains("Some content"),
            "content between hint tags should remain"
        );
    }

    #[test]
    fn clean_markdown_preserves_code_blocks() {
        let input = "```rust\nlet x = 1;\n```";
        let cleaned = clean_markdown_for_terminal(input);
        assert!(
            cleaned.contains("```rust"),
            "code fence start should remain"
        );
        assert!(cleaned.contains("let x = 1;"), "code content should remain");
        assert!(cleaned.contains("```"), "code fence end should remain");
    }

    #[test]
    fn absent_config_returns_default_lint_levels() {
        let dir = std::env::temp_dir().join("cost_lint_test_absent");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let result = parse_budget_config(&dir.join("budget.toml").to_string_lossy());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to read"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unparseable_config_returns_error() {
        let dir = std::env::temp_dir().join("cost_lint_test_unparseable");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("budget.toml");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(file, "this is not valid toml = {{{{{{").unwrap();
        drop(file);

        let result = parse_budget_config(&path.to_string_lossy());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Failed to parse"),
            "expected parse error, got: {}",
            err
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn valid_config_returns_flags() {
        let dir = std::env::temp_dir().join("cost_lint_test_valid");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("budget.toml");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(
            file,
            "[lints]\nsoroban_storage_in_loop = \"deny\"\nredundant_env_clone = \"warn\""
        )
        .unwrap();
        drop(file);

        let result = parse_budget_config(&path.to_string_lossy());
        assert!(result.is_ok());
        let flags = result.unwrap();
        assert_eq!(flags.len(), 2);
        assert!(flags.contains(&"-D soroban_storage_in_loop".to_string()));
        assert!(flags.contains(&"-W redundant_env_clone".to_string()));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_level_returns_error() {
        let dir = std::env::temp_dir().join("cost_lint_test_unknown_level");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("budget.toml");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(file, "[lints]\nsoroban_storage_in_loop = \"oops\"").unwrap();
        drop(file);

        let result = parse_budget_config(&path.to_string_lossy());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Unknown lint level"));

        let _ = fs::remove_dir_all(&dir);
    }

    // The lenient parser (`try_parse_budget_config`) implements issue #191:
    // missing / unreadable / empty / syntactically invalid config files
    // must fall back to safe defaults (empty flag set, stderr warning)
    // instead of aborting. Validation errors — unknown lint names or
    // levels — still propagate so existing tests above keep passing.

    #[test]
    fn try_parse_budget_config_uses_safe_defaults_for_missing_file() {
        let dir = std::env::temp_dir().join("cost_lint_test_lenient_missing");
        let _ = fs::remove_dir_all(&dir);

        let result = try_parse_budget_config(&dir.join("budget.toml").to_string_lossy());
        assert!(
            result.is_ok(),
            "expected safe-default Ok, got: {:?}",
            result
        );
        assert!(
            result.unwrap().is_empty(),
            "expected empty flag set for missing file"
        );
    }
    #[test]
    fn try_parse_budget_config_uses_safe_defaults_for_invalid_toml() {
        let dir = std::env::temp_dir().join("cost_lint_test_lenient_unparseable");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("budget.toml");
        let mut file = fs::File::create(&path).unwrap();
        // Four `{` form two balanced `{{` escapes (two literal `{`
        // characters), keeping the writeln! format string valid while
        // still producing genuinely invalid TOML syntax.
        writeln!(file, "this is not valid toml = {{{{").unwrap();
        drop(file);

        let result = try_parse_budget_config(&path.to_string_lossy());
        assert!(
            result.is_ok(),
            "expected safe-default Ok, got: {:?}",
            result
        );
        assert!(
            result.unwrap().is_empty(),
            "expected empty flag set for invalid TOML"
        );
    }

    #[test]
    fn try_parse_budget_config_uses_safe_defaults_for_empty_file() {
        let dir = std::env::temp_dir().join("cost_lint_test_lenient_empty");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("budget.toml");
        // Create an empty file (no `mut`: never written to).
        let _ = fs::File::create(&path).unwrap();

        let result = try_parse_budget_config(&path.to_string_lossy());
        assert!(
            result.is_ok(),
            "expected safe-default Ok, got: {:?}",
            result
        );
        assert!(
            result.unwrap().is_empty(),
            "expected empty flag set for empty file"
        );
    }

    #[test]
    fn try_parse_budget_config_still_errors_on_unknown_level() {
        // Validation errors must NOT be swallowed by safe-default
        // fallback — a typo'd level is a real user mistake that should
        // stay loud.
        let dir = std::env::temp_dir().join("cost_lint_test_lenient_unknown_level");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("budget.toml");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(file, "[lints]\nsoroban_storage_in_loop = \"oops\"").unwrap();
        drop(file);

        let result = try_parse_budget_config(&path.to_string_lossy());
        assert!(result.is_err(), "expected Err for unknown level");
        assert!(result.unwrap_err().contains("Unknown lint level"));
    }

    // --- CLI argument parser unit tests (issue #320) ---

    #[test]
    fn cli_default_values_when_no_flags() {
        let cli = Cli::try_parse_from(["cargo-cost-lint"]).expect("parsing should succeed");
        assert_eq!(cli.config, None);
        assert!(!cli.list_lints);
        assert_eq!(cli.format, OutputFormat::Text);
    }

    #[test]
    fn cli_parses_config_flag() {
        let cli = Cli::try_parse_from(["cargo-cost-lint", "--config", "my-budget.toml"])
            .expect("parsing should succeed");
        assert_eq!(cli.config, Some("my-budget.toml".to_string()));
    }

    #[test]
    fn cli_parses_list_lints_flag() {
        let cli = Cli::try_parse_from(["cargo-cost-lint", "--list-lints"])
            .expect("parsing should succeed");
        assert!(cli.list_lints);
        assert_eq!(cli.config, None);
        assert_eq!(cli.format, OutputFormat::Text);
    }

    #[test]
    fn cli_parses_format_flag() {
        let cli = Cli::try_parse_from(["cargo-cost-lint", "--format", "json"])
            .expect("parsing should succeed");
        assert_eq!(cli.format, OutputFormat::Json);
    }

    #[test]
    fn cli_parses_multiple_flags() {
        let cli = Cli::try_parse_from([
            "cargo-cost-lint",
            "--config",
            "budget.toml",
            "--format",
            "json",
            "--list-lints",
        ])
        .expect("parsing should succeed");
        assert_eq!(cli.config, Some("budget.toml".to_string()));
        assert_eq!(cli.format, OutputFormat::Json);
        assert!(cli.list_lints);
    }

    #[test]
    fn cli_unknown_flag_returns_error() {
        let result = Cli::try_parse_from(["cargo-cost-lint", "--unknown-flag"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_parses_quiet_flag() {
        let cli =
            Cli::try_parse_from(["cargo-cost-lint", "--quiet"]).expect("parsing should succeed");
        assert!(cli.quiet);
        assert!(!cli.verbose);
    }

    #[test]
    fn cli_parses_verbose_flag() {
        let cli =
            Cli::try_parse_from(["cargo-cost-lint", "--verbose"]).expect("parsing should succeed");
        assert!(cli.verbose);
        assert!(!cli.quiet);
    }

    #[test]
    fn cli_quiet_and_verbose_are_mutually_exclusive() {
        let result = Cli::try_parse_from(["cargo-cost-lint", "--quiet", "--verbose"]);
        assert!(
            result.is_err(),
            "--quiet and --verbose should not be allowed together"
        );
    }

    #[test]
    fn cli_quiet_default_is_false() {
        let cli = Cli::try_parse_from(["cargo-cost-lint"]).expect("parsing should succeed");
        assert!(!cli.quiet);
        assert!(!cli.verbose);
    }
}
