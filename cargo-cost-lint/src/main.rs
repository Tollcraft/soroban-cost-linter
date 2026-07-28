mod module_13;
mod module_15;

use clap::{Parser, ValueEnum};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{exit, Command, Stdio};

mod config;
mod error;

use config::Config;
use error::{LinterError, LinterResult};

/// Output format for lint results.
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

/// SARIF 2.1.0 report root.
#[derive(Serialize)]
struct SarifReport {
    #[serde(rename = "$schema")]
    schema: String,
    version: String,
    runs: Vec<SarifRun>,
}

/// A single SARIF run (one invocation of the linter).
#[derive(Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

/// Tool metadata for SARIF output.
#[derive(Serialize)]
struct SarifTool {
    driver: SarifToolDriver,
}

/// Tool-driver metadata for SARIF output.
#[allow(non_snake_case)]
#[derive(Serialize)]
struct SarifToolDriver {
    name: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "informationUri")]
    information_uri: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rules: Vec<serde_json::Value>,
}

/// A single SARIF result (one lint finding).
#[derive(Serialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: String,
    level: String,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

/// SARIF message text.
#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

/// SARIF location referencing a physical file.
#[derive(Serialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    physical_location: SarifPhysicalLocation,
}

/// SARIF physical location in a file.
#[derive(Serialize)]
struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: SarifArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<SarifRegion>,
}

/// SARIF artifact location (file URI).
#[derive(Serialize)]
struct SarifArtifactLocation {
    uri: String,
}

/// SARIF region (line/column range within a file).
#[derive(Serialize)]
struct SarifRegion {
    #[serde(rename = "startLine")]
    start_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "startColumn")]
    start_column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "endLine")]
    end_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "endColumn")]
    end_column: Option<usize>,
}

/// CLI arguments for `cargo-cost-lint`.
///
/// This struct is parsed by `clap` from the command-line invocation.  It
/// wraps `cargo dylint` with additional filtering, formatting, and
/// fix-application logic.
#[derive(Parser, Debug)]
#[command(name = "cargo-cost-lint")]
#[command(version)]
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
include!(concat!(env!("OUT_DIR"), "/lint_metadata.rs"));

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

/// Budget config deserialized from budget.toml — used by the
/// `parse_budget_config` test helper and `load_budget_config`.
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct BudgetConfig {
    lints: Option<HashMap<String, String>>,
}

#[allow(dead_code)]
fn load_budget_config(config_path: &Path) -> Option<BudgetConfig> {
    if !config_path.exists() {
        return None;
    }

    let config_str = fs::read_to_string(config_path).ok()?;
    toml::from_str::<BudgetConfig>(&config_str).ok()
}

#[allow(dead_code)]
fn parse_budget_config(path: &str) -> Result<Vec<String>, String> {
    let config_str =
        fs::read_to_string(path).map_err(|e| format!("Error: Failed to read {}: {}", path, e))?;
    let config: BudgetConfig = toml::from_str(&config_str)
        .map_err(|e| format!("Error: Failed to parse {}: {}", path, e))?;
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

/// Loads `path` as a validated `BudgetConfig` and formats its `[lints]`
/// entries into `-A`/`-W`/`-D` flags for `DYLINT_RUSTFLAGS`. Validation
/// (unknown lint names, invalid levels) is handled by
/// `BudgetConfig::from_file_validated`, the single canonical config parser.
fn parse_budget_config(path: &str) -> Result<Vec<String>, String> {
    let config = BudgetConfig::from_file_validated(Path::new(path), LINT_NAMES)?;

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
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        exit(1);
    }
}

/// Core logic extracted into a fallible function so that all error sites can
/// use the `?` operator rather than a mix of `unwrap`, `expect`, and `exit`.
fn run() -> LinterResult<()> {
    let mut args = std::env::args().collect::<Vec<_>>();
    if args.len() > 1 && args[1] == "cost-lint" {
        args.remove(1);
    }

    // Handle --version / -V explicitly: clap 4.0's #[command(version)]
    // displays the version but does not auto-exit, so we must check early
    // and exit 0 before falling through to the cargo-dylint machinery.
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("cargo-cost-lint {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => {
            e.print()?;
            return Err(LinterError::Other("argument parsing failed".into()));
        }
    };

    if cli.list_lints {
        if cli.format == OutputFormat::Json {
            println!("{}", serde_json::to_string_pretty(&LINT_INVENTORY).unwrap());
        } else {
            println!("Lint inventory (version {}):", LINT_INVENTORY.version);
            for lint in LINT_INVENTORY.lints {
                println!("{} | {} | {} | {}", lint.name, lint.default_level, lint.category, lint.documentation_url);
            }
        }
        return Ok(());
    }

    let mut lint_flags = Vec::new();

    let lint_flags = if let Some(config_path) = &cli.config {
        let config = Config::from_file_or_default(Path::new(config_path))?;
        config.to_lint_flags()
    } else {
        Vec::new()
    };

    let preflight = Command::new("cargo")
        .arg("dylint")
        .arg("--version")
        .output();

    match preflight {
        Ok(output) if output.status.success() => {}
        _ => {
            eprintln!("Error: cargo-dylint is not installed or not available in PATH.");
            eprintln!("Please install it by running:");
            eprintln!("    cargo install cargo-dylint dylint-link");
            exit(1);
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

    if cli.format == OutputFormat::Json {
        cmd.arg("--");
        cmd.arg("--message-format=json");
        cmd.stdout(Stdio::piped());
    }

    let mut child = cmd.spawn().map_err(|e| {
        LinterError::MissingPrerequisite(format!(
            "Failed to execute cargo dylint: {}. Is cargo-dylint installed?",
            e
        ))
    })?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| LinterError::Other("Failed to capture stdout".into()))?;
    let reader = BufReader::new(stdout);
    let mut highest_exit_code = 0;
    let mut lint_counts: HashMap<String, usize> = HashMap::new();

    let mut findings: Vec<LintFinding> = Vec::new();

    for line_str in reader.lines().map_while(Result::ok) {
        if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line_str)
            && msg.get("reason").and_then(|r| r.as_str()) == Some("compiler-message")
            && let Some(message) = msg.get("message")
            && let Some(code) = message.get("code")
            && let Some(lint_name) = code.get("code").and_then(|c| c.as_str())
            && LINT_NAMES.contains(&lint_name)
        {
            let level = message
                .get("level")
                .and_then(|l| l.as_str())
                .unwrap_or("unknown");

            let diagnostic_message = message
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("");
            let mut file = String::new();
            let mut primary_span = Span {
                line_start: 0,
                line_end: 0,
                column_start: 0,
                column_end: 0,
            };

            if let Some(spans) = message.get("spans").and_then(|s| s.as_array()) {
                for span in spans {
                    if span
                        .get("is_primary")
                        .and_then(|p| p.as_bool())
                        .unwrap_or(false)
                    {
                        file = span
                            .get("file_name")
                            .and_then(|f| f.as_str())
                            .unwrap_or("")
                            .to_string();
                        primary_span.line_start =
                            span.get("line_start").and_then(|l| l.as_u64()).unwrap_or(0) as usize;
                        primary_span.line_end =
                            span.get("line_end").and_then(|l| l.as_u64()).unwrap_or(0) as usize;
                        primary_span.column_start =
                            span.get("column_start")
                                .and_then(|c| c.as_u64())
                                .unwrap_or(0) as usize;
                        primary_span.column_end =
                            span.get("column_end").and_then(|c| c.as_u64()).unwrap_or(0) as usize;
                        break;
                    }
                }
            }

            if !is_reportable(&file, &allowed) {
                continue;
            }

            *lint_counts.entry(lint_name.to_string()).or_insert(0) += 1;

            if level == "error" || level == "deny" {
                highest_exit_code = 1;
            }

            let mut help_text = None;
            let mut suggestion = None;
            if let Some(children) = message.get("children").and_then(|c| c.as_array()) {
                for child_item in children {
                    if child_item.get("level").and_then(|l| l.as_str()) == Some("help") {
                        let child_msg = child_item
                            .get("message")
                            .and_then(|m| m.as_str())
                            .map(|s| s.to_string());
                        help_text = child_msg.clone();
                        if cli.fix {
                            suggestion = extract_suggestion(&child_msg, lint_name);
                        }
                        break;
                    }
                }
            }

            let finding = LintFinding {
                name: lint_name.to_string(),
                level: level.to_string(),
                file: file.clone(),
                span: primary_span,
                message: diagnostic_message.to_string(),
                help: help_text,
                suggestion,
            };

            findings.push(finding);

            if cli.format == OutputFormat::Json {
                // Safe: we just pushed an element above.
                let finding_json = findings.last().unwrap();
                if let Ok(json_str) = serde_json::to_string(finding_json) {
                    println!("{}", json_str);
                }
            } else if cli.format != OutputFormat::Sarif {
                let rendered = message
                    .get("rendered")
                    .and_then(|r| r.as_str())
                    .unwrap_or(diagnostic_message);
                print!("{}", rendered);
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

    let status = child.wait()?;

    // Print per-lint summary to stderr when there are findings
    if !lint_counts.is_empty() {
        eprintln!("lint summary:");
        let mut sorted: Vec<_> = lint_counts.iter().collect();
        sorted.sort_by_key(|(name, _)| *name);
        for (name, count) in &sorted {
            eprintln!("  {}: {}", name, count);
        }
        let total: usize = lint_counts.values().sum();
        eprintln!("total: {}", total);
    }

    if !status.success() {
        return Err(LinterError::Subprocess {
            code: status.code(),
        });
    } else if highest_exit_code != 0 {
        exit(highest_exit_code);
    }

    Ok(())
}

/// Prints the explanation for a lint, or errors with valid lint names if not found.
fn print_explanation(lint_name: &str) {
    let normalized = lint_name.to_lowercase();

    let explanation = LINT_EXPLANATIONS
        .iter()
        .find(|e| e.name == normalized);

    match explanation {
        Some(entry) => {
            // Clean up the markdown for terminal display
            let cleaned = clean_markdown_for_terminal(entry.markdown);
            println!("{}", cleaned);
        }
        None => {
            eprintln!(
                "Error: unknown lint '{}'.\n\nValid lints:\n",
                lint_name
            );
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

    for (file_path, edits) in &file_edits {
        match fs::read_to_string(file_path) {
            Ok(content) => {
                let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
                for (line_idx, _message, suggestion) in edits {
                    if *line_idx > 0 && *line_idx <= lines.len() {
                        let line = &mut lines[*line_idx - 1];
                        if let Some(start) = line.find("Symbol::new")
                            && let Some(end) = line[start..].find(')')
                        {
                            let replace_end = start + end + 1;
                            line.replace_range(start..replace_end, suggestion);
                        }
                    }
                }
                let new_content = lines.join("\n");
                if let Err(e) = fs::write(file_path, new_content) {
                    eprintln!("Failed to write {}: {}", file_path, e);
                }
            }
            Err(e) => {
                eprintln!("Failed to read {}: {}", file_path, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;

    use super::*;

    fn write_file(path: &Path, contents: &str) {
        let mut f = File::create(path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
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
            let explanation = LINT_EXPLANATIONS
                .iter()
                .find(|e| e.name == info.name);
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
        let explanation = LINT_EXPLANATIONS
            .iter()
            .find(|e| e.name == first.name);
        assert!(
            explanation.is_some(),
            "lint '{}' should have explanation",
            first.name
        );
        let entry = explanation.unwrap();
        assert!(!entry.markdown.is_empty(), "explanation should not be empty");
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
        assert!(cleaned.contains("```rust"), "code fence start should remain");
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
        writeln!(file, "this is not valid toml = {{{{").unwrap();
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
}
