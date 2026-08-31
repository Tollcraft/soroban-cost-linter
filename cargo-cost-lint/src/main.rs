use clap::Parser;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

mod cache;
mod config;
mod diff_files;
mod error;
mod lint_name_set;
mod output_formatters;

use error::{LinterError, LinterResult};
use lint_name_set::LintNameSet;
use output_formatters::{OutputFormat, LintFinding};

#[derive(Parser, Debug, Clone)]
#[command(
    name = "cargo-cost-lint",
    bin_name = "cargo-cost-lint",
    author,
    version,
    about = "The static analysis shield for Soroban smart contracts"
)]
pub struct Cli {
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    #[arg(long)]
    pub list_lints: bool,

    #[arg(long)]
    pub cache: bool,

    #[arg(long, default_value = "target/cargo-cost-lint-cache")]
    pub cache_dir: PathBuf,

    #[arg(long)]
    pub diff_only: bool,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub cargo_args: Vec<String>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

fn run() -> LinterResult<()> {
    let cli = Cli::parse();

    if cli.list_lints {
        list_lints(&cli)?;
        return Ok(());
    }

    // Run underlying cargo check via dylint and parse findings
    let findings = execute_dylint_and_collect_findings(&cli)?;
    let mut stdout = io::stdout();
    let mut findings_acc = Vec::new();

    for finding in &findings {
        output_formatters::handle_finding(&cli, finding, &mut findings_acc, &mut stdout)?;
    }

    output_formatters::print_findings_summary(&cli.format, &findings_acc, &mut stdout)?;

    if !findings.is_empty() {
        // Check if any finding is deny-level or error-level
        let has_errors = findings.iter().any(|f| f.level == "error" || f.level == "deny");
        if has_errors {
            std::process::exit(1);
        }
    }

    Ok(())
}

fn list_lints(cli: &Cli) -> LinterResult<()> {
    let inventory = lint_name_set::get_lint_inventory();

    if cli.format == OutputFormat::Json {
        let json = serde_json::to_string_pretty(&inventory)
            .map_err(|e| LinterError::Other(e.to_string()))?;
        println!("{}", json);
    } else {
        println!("Lint inventory (version {}):", inventory.version);
        for lint in &inventory.lints {
            println!("  - {}: [{}] {}", lint.name, lint.category, lint.description);
            println!(
                "    Default level: {}, Docs: {}",
                lint.default_level, lint.documentation_url
            );
        }
    }

    Ok(())
}

fn execute_dylint_and_collect_findings(cli: &Cli) -> LinterResult<Vec<LintFinding>> {
    // In real execution, cargo-cost-lint invokes cargo dylint.
    // For integration tests and real runs, we check if we have stubbed/simulated execution or run cargo dylint.
    // When invoked as `cargo cost-lint`, cargo passes "cost-lint" as the first argument in cli.cargo_args.
    let mut args = cli.cargo_args.clone();
    if args.first().map(|s| s.as_str()) == Some("cost-lint") {
        args.remove(0);
    }

    // If cache is requested, check cache first
    if cli.cache {
        if let Some(cached) = cache::load_from_cache(&cli.cache_dir, &args) {
            return Ok(cached);
        }
    }

    // Execute cargo check with dylint library
    let findings = invoke_dylint(&args)?;

    if cli.cache {
        cache::save_to_cache(&cli.cache_dir, &args, &findings)?;
    }

    if cli.diff_only {
        filter_findings_to_changed_files(&findings)
    } else {
        Ok(findings)
    }
}

/// Filter findings to only include those in files changed relative to the
/// default branch merge base. Lints the entire changed file, not just
/// changed lines.
fn filter_findings_to_changed_files(findings: &[LintFinding]) -> LinterResult<Vec<LintFinding>> {
    let changed = diff_files::get_changed_files()?;
    if changed.is_empty() {
        eprintln!("--diff-only: no changed files found; nothing to lint.");
        return Ok(vec![]);
    }
    eprintln!("--diff-only: linting {} changed file(s)", changed.len());
    let cwd = std::env::current_dir().unwrap_or_default();
    let filtered: Vec<LintFinding> = findings
        .iter()
        .filter(|f| {
            let path = std::path::Path::new(&f.file);
            let relative = path.strip_prefix(&cwd).unwrap_or(path);
            let relative_str = relative.to_string_lossy().to_string();
            changed.iter().any(|c| relative_str == *c || relative_str.ends_with(c))
        })
        .cloned()
        .collect();
    Ok(filtered)
}

fn invoke_dylint(args: &[String]) -> LinterResult<Vec<LintFinding>> {
    use std::process::Command;

    // Determine DYLINT_LIBRARY_PATH or build soroban_cost_lints if needed.
    // If we are running tests or cargo-cost-lint, invoke cargo dylint check.
    let mut cmd = Command::new("cargo");
    cmd.arg("dylint").arg("rustc");

    // Forward user args
    cmd.args(args);

    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                return Err(LinterError::MissingPrerequisite(
                    "error: `cargo-dylint` is not installed.
To install it, run:
    cargo install cargo-dylint dylint-link --version \"^6.0.1\""
                        .to_string(),
                ));
            }
            return Err(LinterError::Io(e));
        }
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Parse findings from stderr/stdout where cargo dylint / compiler outputs json or diagnostic messages.
    // In dylint integration, cargo-cost-lint captures compiler JSON messages or structured diagnostic lines.
    let findings = parse_dylint_output(&stderr);

    Ok(findings)
}

fn parse_dylint_output(output: &str) -> Vec<LintFinding> {
    let mut findings = Vec::new();

    // Parse compiler diagnostic json or stderr output lines
    for line in output.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if json["reason"] == "compiler-message" {
                if let Some(msg) = json["message"].as_object() {
                    if let Some(code) = msg.get("code").and_then(|c| c.get("code")) {
                        let lint_name = code.as_str().unwrap_or("").to_string();
                        // Check if it's one of our known lints or has soroban/cost prefix
                        if !lint_name.is_empty() {
                            let level = msg.get("level").and_then(|l| l.as_str()).unwrap_or("warning").to_string();
                            let message = msg.get("message").and_then(|m| m.as_str()).unwrap_or("").to_string();
                            let span_obj = msg.get("spans").and_then(|s| s.as_array()).and_then(|arr| arr.first());

                            let (file, line_start, line_end, col_start, col_end) = if let Some(sp) = span_obj {
                                (
                                    sp.get("file_name").and_then(|f| f.as_str()).unwrap_or("").to_string(),
                                    sp.get("line_start").and_then(|l| l.as_u64()).unwrap_or(0) as usize,
                                    sp.get("line_end").and_then(|l| l.as_u64()).unwrap_or(0) as usize,
                                    sp.get("column_start").and_then(|c| c.as_u64()).unwrap_or(0) as usize,
                                    sp.get("column_end").and_then(|c| c.as_u64()).unwrap_or(0) as usize,
                                )
                            } else {
                                ("".to_string(), 0, 0, 0, 0)
                            };

                            findings.push(LintFinding {
                                name: lint_name,
                                level,
                                file,
                                span: output_formatters::Span {
                                    line_start,
                                    line_end,
                                    column_start: col_start,
                                    column_end: col_end,
                                },
                                message,
                                help: None,
                                suggestion: None,
                            });
                        }
                    }
                }
            }
        }
    }

    findings
}
