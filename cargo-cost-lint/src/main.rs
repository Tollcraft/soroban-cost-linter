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

/// Root schema for `budget.toml`, shared with `soroban-budget-assert`.
///
/// **Foreign sections:** `[network]`, `[source]`, and `[functions.*]` are owned
/// by `soroban-budget-assert`.  They are intentionally left untyped here so
/// that a file containing both tools' sections parses without error.
#[derive(Deserialize, Debug)]
struct BudgetConfig {
    /// Lint-level overrides for `soroban-cost-linter`.
    /// Unknown keys inside this table are rejected at deserialization time.
    lints: Option<LintsConfig>,
}

/// The `[lints]` table inside `budget.toml`.
///
/// Every key is an optional lint-name → severity mapping.  `deny_unknown_fields`
/// ensures that a typo in a lint name produces a clear serde error rather than
/// being silently ignored.
///
/// **Keep in sync** with the lint registrations in
/// `soroban_cost_lints/src/lib.rs` (the source of truth for valid lint names).
/// When a new lint is added there, a corresponding `Option<String>` field
/// must be added here.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct LintsConfig {
    soroban_storage_in_loop: Option<String>,
    redundant_env_clone: Option<String>,
    unnecessary_host_function_call: Option<String>,
}

const VALID_LEVELS: &[&str] = &["allow", "warn", "deny"];

/// Validate that `level` is one of the recognised severity levels.
/// Returns `Ok(())` for valid levels and `Err(String)` with a human-
/// readable message for invalid ones.
fn validate_lint_level(lint_name: &str, level: &str) -> Result<(), String> {
    if !VALID_LEVELS.contains(&level) {
        Err(format!(
            "invalid level '{}' for lint '{}'. Valid levels are: {}",
            level,
            lint_name,
            VALID_LEVELS.join(", ")
        ))
    } else {
        Ok(())
    }
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
            let config_str = fs::read_to_string(config_path).unwrap_or_else(|e| {
                eprintln!("Error reading {}: {}", config_path, e);
                exit(1);
            });
            let config = toml::from_str::<BudgetConfig>(&config_str).unwrap_or_else(|e| {
                eprintln!("Error parsing {}: {}", config_path, e);
                exit(1);
            });
            if let Some(ref lints) = config.lints {
                if let Some(ref level) = lints.soroban_storage_in_loop {
                    if let Err(e) = validate_lint_level("soroban_storage_in_loop", level) {
                        eprintln!("Error: {}", e);
                        exit(1);
                    }
                }
                if let Some(ref level) = lints.redundant_env_clone {
                    if let Err(e) = validate_lint_level("redundant_env_clone", level) {
                        eprintln!("Error: {}", e);
                        exit(1);
                    }
                }
                if let Some(ref level) = lints.unnecessary_host_function_call {
                    if let Err(e) = validate_lint_level("unnecessary_host_function_call", level) {
                        eprintln!("Error: {}", e);
                        exit(1);
                    }
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

                                if cli.format == OutputFormat::Json {
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

    let status = child.wait().expect("Failed to wait on cargo dylint");
    if !status.success() {
        exit(status.code().unwrap_or(1));
    } else if highest_exit_code != 0 {
        exit(highest_exit_code);
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

    // ── budget.toml schema tests ─────────────────────────────────────────

    #[test]
    fn budget_config_parses_valid_lints() {
        let toml_str = r#"
[lints]
soroban_storage_in_loop = "deny"
redundant_env_clone = "warn"
"#;
        let config = toml::from_str::<BudgetConfig>(toml_str).unwrap();
        let lints = config.lints.unwrap();
        assert_eq!(
            lints.soroban_storage_in_loop.as_deref(),
            Some("deny")
        );
        assert_eq!(
            lints.redundant_env_clone.as_deref(),
            Some("warn")
        );
        assert_eq!(lints.unnecessary_host_function_call.as_deref(), None);
    }

    #[test]
    fn budget_config_accepts_foreign_top_level_sections() {
        // soroban-budget-assert owns [network], [source], and [functions.*].
        // These must parse cleanly alongside our [lints] section.
        let toml_str = r#"
[lints]
soroban_storage_in_loop = "deny"

[network]
rpc_url = "https://soroban-testnet.stellar.org"

[source]
account = "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"

[functions.example]
max_cpu_instructions = 100_000_000
"#;
        let config = toml::from_str::<BudgetConfig>(toml_str).unwrap();
        assert!(config.lints.is_some());
    }

    #[test]
    fn budget_config_rejects_unknown_lint_name() {
        let toml_str = r#"
[lints]
soroban_storage_in_loopp = "deny"
"#;
        let err = toml::from_str::<BudgetConfig>(toml_str).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("soroban_storage_in_loopp"),
            "error should name the offending key, got: {msg}"
        );
    }

    #[test]
    fn budget_config_no_lints_section_parses() {
        let toml_str = r#"
[network]
rpc_url = "https://soroban-testnet.stellar.org"
"#;
        let config = toml::from_str::<BudgetConfig>(toml_str).unwrap();
        assert!(config.lints.is_none());
    }

    #[test]
    fn budget_config_empty_lints_section_parses() {
        let toml_str = "[lints]\n";
        let config = toml::from_str::<BudgetConfig>(toml_str).unwrap();
        let lints = config.lints.unwrap();
        assert_eq!(lints.soroban_storage_in_loop.as_deref(), None);
        assert_eq!(lints.redundant_env_clone.as_deref(), None);
        assert_eq!(lints.unnecessary_host_function_call.as_deref(), None);
    }

    #[test]
    fn validate_lint_level_accepts_allow_warn_deny() {
        assert!(validate_lint_level("test_lint", "allow").is_ok());
        assert!(validate_lint_level("test_lint", "warn").is_ok());
        assert!(validate_lint_level("test_lint", "deny").is_ok());
    }

    #[test]
    fn validate_lint_level_rejects_invalid_level() {
        let err = validate_lint_level("test_lint", "invalid").unwrap_err();
        assert!(err.contains("invalid"), "error should name the bad level: {err}");
        assert!(err.contains("test_lint"), "error should name the lint: {err}");
        assert!(
            err.contains("allow"),
            "error should list valid levels: {err}"
        );
    }
}
