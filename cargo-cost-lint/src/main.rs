pub mod cache;
mod config;
mod error;
#[allow(dead_code)]
mod lint_name_set;
mod output_formatters;

use clap::{ArgGroup, Parser, ValueEnum};
use config::BudgetConfig;
use output_formatters::{LintFinding, OutputFormat, Span};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, exit};
#[derive(Parser, Debug)]
#[command(name = "cargo-cost-lint")]
#[command(version = long_version())]
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

    #[arg(
        long = "allow",
        short = 'A',
        value_name = "LINT",
        action = clap::ArgAction::Append,
        help = "Allow a lint for this run (overrides budget.toml)"
    )]
    allow: Vec<String>,

    #[arg(
        long = "warn",
        short = 'W',
        value_name = "LINT",
        action = clap::ArgAction::Append,
        help = "Set a lint to warning for this run (overrides budget.toml)"
    )]
    warn: Vec<String>,

    #[arg(
        long = "deny",
        short = 'D',
        value_name = "LINT",
        action = clap::ArgAction::Append,
        help = "Deny a lint for this run (overrides budget.toml)"
    )]
    deny: Vec<String>,

    #[arg(
        long = "package",
        short = 'p',
        value_name = "SPEC",
        action = clap::ArgAction::Append,
        help = "Package(s) to lint (repeatable)"
    )]
    package: Vec<String>,

    #[arg(long = "workspace", help = "Lint all packages in the workspace")]
    workspace: bool,

    #[arg(long = "no-cache", help = "Bypass the lint result cache for this run")]
    no_cache: bool,

    #[arg(long = "clear-cache", help = "Clear the lint result cache and exit")]
    clear_cache: bool,

    /// Control coloured output: auto, always, never.
    ///
    /// When set to *auto* (the default), colour is enabled only when
    /// standard output is a terminal.  Honours the widely-adopted
    /// `NO_COLOR` convention (https://no-color.org/): if this flag is
    /// omitted and the `NO_COLOR` environment variable is set to any
    /// non-empty value, output is uncoloured.
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto, value_name = "WHEN")]
    color: ColorChoice,

    #[arg(
        long,
        help = "Path to baseline file for suppressing pre-existing findings"
    )]
    baseline: Option<String>,

    #[arg(
        long,
        help = "Update or create the baseline file with current findings"
    )]
    bless: bool,

    #[arg(long, help = "Automatically apply machine-applicable suggestions")]
    fix: bool,

    #[arg(
        long,
        help = "Allow --fix to run on a dirty working tree with unstaged changes"
    )]
    allow_dirty: bool,

    #[arg(
        long,
        help = "Allow --fix to run on a dirty working tree with staged changes"
    )]
    allow_staged: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BaselineFinding {
    pub lint_name: String,
    pub file: String,
    pub context_hash: String,
    pub code_snippet: String,
    pub occurrence: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Baseline {
    pub version: u32,
    pub findings: Vec<BaselineFinding>,
}

pub fn normalize_file_path(path_str: &str) -> String {
    if let Ok(current_dir) = std::env::current_dir() {
        let p = Path::new(path_str);
        if let Ok(stripped) = p.strip_prefix(&current_dir) {
            return stripped.to_string_lossy().replace('\\', "/");
        }
    }
    path_str.replace('\\', "/")
}

pub fn compute_context_hash(lint_name: &str, relative_file: &str, context_str: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    lint_name.hash(&mut hasher);
    relative_file.hash(&mut hasher);
    context_str.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub fn extract_finding_context(
    file_path: &str,
    line_start: usize,
    line_end: usize,
) -> (String, String) {
    if line_start > 0
        && let Ok(content) = fs::read_to_string(file_path)
    {
        let lines: Vec<&str> = content.lines().collect();
        if line_start <= lines.len() {
            let snippet_start = line_start - 1;
            let snippet_end = line_end.min(lines.len());
            let snippet = lines[snippet_start..snippet_end].join("\n");

            let ctx_start = if line_start > 1 { line_start - 2 } else { 0 };
            let ctx_end = (line_end + 1).min(lines.len());
            let context = lines[ctx_start..ctx_end].join("\n");

            return (snippet, context);
        }
    }
    (String::new(), String::new())
}

fn check_git_clean(allow_dirty: bool, allow_staged: bool) -> Result<(), String> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map_err(|e| format!("Error checking git status: {}", e))?;

    if !output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    if lines.is_empty() {
        return Ok(());
    }

    let mut has_staged = false;
    let mut has_unstaged = false;

    for line in lines {
        let status_code = &line[0..2.min(line.len())];
        let x = status_code.chars().next().unwrap_or(' ');
        let y = status_code.chars().nth(1).unwrap_or(' ');

        if x != ' ' && x != '?' {
            has_staged = true;
        }
        if y != ' ' {
            has_unstaged = true;
        }
    }

    if has_unstaged && !allow_dirty {
        return Err("error: the working tree has dirty files, aborting. Pass --allow-dirty to ignore dirty working tree.".to_string());
    }
    if has_staged && !allow_staged && !allow_dirty {
        return Err("error: the working tree has staged changes, aborting. Pass --allow-staged or --allow-dirty to ignore.".to_string());
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct MachineFix {
    file_path: String,
    line_start: usize,
    column_start: usize,
    line_end: usize,
    column_end: usize,
    replacement: String,
    _lint_name: String,
}

fn apply_machine_fixes(fixes: &[MachineFix]) -> Result<(usize, usize), String> {
    if fixes.is_empty() {
        return Ok((0, 0));
    }

    let mut file_map: std::collections::HashMap<String, Vec<&MachineFix>> =
        std::collections::HashMap::new();
    for fix in fixes {
        file_map.entry(fix.file_path.clone()).or_default().push(fix);
    }

    let file_count = file_map.len();
    let mut applied_count = 0;

    for (file_path, mut fix_list) in file_map {
        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => return Err(format!("Error reading {}: {}", file_path, e)),
        };

        let get_offset = |content: &str, line: usize, col: usize| -> usize {
            let mut curr_line = 1;
            let mut curr_col = 1;
            for (idx, ch) in content.char_indices() {
                if curr_line == line && curr_col == col {
                    return idx;
                }
                if ch == '\n' {
                    curr_line += 1;
                    curr_col = 1;
                } else {
                    curr_col += 1;
                }
            }
            content.len()
        };

        fix_list.sort_by(|a, b| {
            let off_a = get_offset(&content, a.line_start, a.column_start);
            let off_b = get_offset(&content, b.line_start, b.column_start);
            off_b.cmp(&off_a)
        });

        let mut new_content = content.clone();
        for fix in fix_list {
            let start = get_offset(&new_content, fix.line_start, fix.column_start);
            let end = get_offset(&new_content, fix.line_end, fix.column_end);
            if start <= end && end <= new_content.len() {
                new_content.replace_range(start..end, &fix.replacement);
                applied_count += 1;
            }
        }

        if let Err(e) = fs::write(&file_path, new_content) {
            return Err(format!("Error writing {}: {}", file_path, e));
        }
    }

    Ok((applied_count, file_count))
}

/// Colour-policy preference forwarded to the underlying `cargo dylint`
/// (and therefore `rustc`) invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ColorChoice {
    /// Emit ANSI colours only when stdout is a terminal (default behaviour).
    Auto,
    /// Always emit ANSI colours, even when piped or redirected.
    Always,
    /// Never emit ANSI colours.
    Never,
}

impl ColorChoice {
    /// Return the `cargo dylint` / `cargo check` `--color` argument
    /// value, or `None` when we should let cargo pick its default
    /// (i.e. when colour is desired and there is nothing to override).
    fn as_cargo_arg(&self) -> Option<&'static str> {
        match self {
            ColorChoice::Auto => None,
            ColorChoice::Always => Some("always"),
            ColorChoice::Never => Some("never"),
        }
    }
}

/// Determine the effective colour preference by merging:
///
/// 1. An explicit `--color` CLI flag (highest priority).
/// 2. The `NO_COLOR` environment variable (set to any non-empty value
///    means "no colour").
/// 3. Cargo's built-in default (`Auto`) — colour when stdout is a
///    terminal, no colour otherwise.
fn resolve_color_choice(cli_color: &ColorChoice) -> ColorChoice {
    match cli_color {
        ColorChoice::Auto => {
            // Honour the NO_COLOR convention (https://no-color.org/).
            if std::env::var("NO_COLOR").is_ok_and(|v| !v.is_empty()) {
                ColorChoice::Never
            } else {
                ColorChoice::Auto
            }
        }
        other => *other,
    }
}

include!(concat!(env!("OUT_DIR"), "/lint_names.rs"));
include!(concat!(env!("OUT_DIR"), "/lint_metadata.rs"));
include!(concat!(env!("OUT_DIR"), "/lint_info.rs"));
include!(concat!(env!("OUT_DIR"), "/lint_explanations.rs"));
include!(concat!(env!("OUT_DIR"), "/version_info.rs"));

/// Build the `--version` string.
///
/// First line is the conventional `name version` shape.
/// Subsequent lines report the pinned nightly toolchain and the
/// expected cargo-dylint version constraint — the two pieces of
/// information most often needed when triaging build or lint issues.
///
/// Leaking the `String` is acceptable here: `--version` is printed
/// once per process invocation and the memory is reclaimed on exit.
fn long_version() -> &'static str {
    Box::leak(
        format!(
            "{}\ntoolchain: {}\ncargo-dylint: {}",
            env!("CARGO_PKG_VERSION"),
            PINNED_TOOLCHAIN,
            DYLINT_VERSION_CONSTRAINT,
        )
        .into_boxed_str(),
    )
}

/// Validates package selection options against available workspace members
/// and constructs the command-line arguments to forward to cargo.
pub fn validate_and_build_package_args(
    packages: &[String],
    workspace: bool,
    available_packages: &[String],
) -> Result<Vec<String>, String> {
    if workspace && !packages.is_empty() {
        return Err(
            "Error: The argument '--workspace' cannot be used with '--package <SPEC>'".to_string(),
        );
    }

    if workspace {
        return Ok(vec!["--workspace".to_string()]);
    }

    if packages.is_empty() {
        return Ok(Vec::new());
    }

    for pkg in packages {
        if !available_packages.iter().any(|p| p == pkg) {
            let valid = available_packages.join(", ");
            return Err(format!(
                "Error: Package '{}' not found in workspace. Valid workspace members are: {}",
                pkg, valid
            ));
        }
    }

    let mut args = Vec::new();
    for pkg in packages {
        args.push("--package".to_string());
        args.push(pkg.clone());
    }
    Ok(args)
}

/// Parses the output of `cargo metadata` to extract workspace member package names.
pub fn parse_workspace_members_from_metadata(json_bytes: &[u8]) -> Result<Vec<String>, String> {
    let value: serde_json::Value = serde_json::from_slice(json_bytes)
        .map_err(|e| format!("Error: Failed to parse `cargo metadata` JSON: {}", e))?;

    let workspace_members = value
        .get("workspace_members")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();

    let packages = value
        .get("packages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Error: `cargo metadata` missing `packages` array".to_string())?;

    let mut member_names: Vec<String> = packages
        .iter()
        .filter_map(|pkg| {
            let id = pkg.get("id").and_then(|i| i.as_str())?;
            let name = pkg.get("name").and_then(|n| n.as_str())?;
            if workspace_members.is_empty() || workspace_members.contains(id) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();

    member_names.sort();
    member_names.dedup();
    Ok(member_names)
}

/// Discovers workspace package members by running `cargo metadata`.
pub fn get_workspace_packages(dir: Option<&Path>) -> Result<Vec<String>, String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["metadata", "--no-deps", "--format-version", "1"]);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("Error: Failed to run `cargo metadata`: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Error: `cargo metadata` failed: {}", stderr.trim()));
    }

    parse_workspace_members_from_metadata(&output.stdout)
}

/// Combines lint configurations from an optional `BudgetConfig` (budget.toml)
/// with command-line overrides (`--allow`, `--warn`, `--deny`).
///
/// Command-line overrides take precedence over `budget.toml`.
/// Unknown lint names in either source produce an error listing all valid lints.
/// Conflicting command-line overrides for the same lint produce an error.
pub fn build_effective_lint_flags(
    config: Option<&BudgetConfig>,
    cli_allow: &[String],
    cli_warn: &[String],
    cli_deny: &[String],
) -> Result<Vec<String>, String> {
    // 1. Process and validate CLI overrides.
    let mut cli_levels: std::collections::HashMap<String, (&'static str, &'static str)> =
        std::collections::HashMap::new();

    let cli_groups = [
        (cli_allow, "allow", "-A"),
        (cli_warn, "warn", "-W"),
        (cli_deny, "deny", "-D"),
    ];

    for (lints, level_name, flag) in cli_groups {
        for lint in lints {
            if !LINT_NAMES.contains(&lint.as_str()) {
                let valid = LINT_NAMES.join(", ");
                return Err(format!(
                    "Error: Unknown lint name '{}'. Valid lints are: {}",
                    lint, valid
                ));
            }
            if let Some((existing_level, _)) = cli_levels.get(lint) {
                if *existing_level != level_name {
                    return Err(format!(
                        "Error: Conflicting lint levels specified for '{}': cannot set to both '{}' and '{}'",
                        lint, existing_level, level_name
                    ));
                }
            } else {
                cli_levels.insert(lint.clone(), (level_name, flag));
            }
        }
    }

    // 2. Process budget.toml configuration if present.
    let mut effective_flags: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();

    if let Some(lints) = config.and_then(|cfg| cfg.lints.as_ref()) {
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
            effective_flags.insert(lint.clone(), format!("{} {}", level_flag, lint));
        }
    }

    // 3. Apply CLI overrides (taking precedence over budget.toml).
    for (lint, (_level_name, flag)) in cli_levels {
        effective_flags.insert(lint.clone(), format!("{} {}", flag, lint));
    }

    Ok(effective_flags.into_values().collect())
}

#[allow(dead_code)]
fn validate_and_build_flags(config: &BudgetConfig) -> Result<Vec<String>, String> {
    build_effective_lint_flags(Some(config), &[], &[], &[])
}

/// Container for `.lintignore` suppression patterns.
#[derive(Clone, Debug)]
pub struct LintIgnore {
    gitignore: ignore::gitignore::Gitignore,
    pub path: PathBuf,
}

impl LintIgnore {
    /// Discovers `.lintignore` by searching `cwd` and walking up to `workspace_root`.
    #[allow(clippy::collapsible_if)]
    pub fn discover(cwd: &Path, workspace_root: &Path) -> Option<Self> {
        let mut current = cwd.to_path_buf();
        loop {
            let path = current.join(".lintignore");
            if path.exists() {
                let mut builder = ignore::gitignore::GitignoreBuilder::new(&current);
                if builder.add(&path).is_none() {
                    let build_res = builder.build();
                    if let Ok(gitignore) = build_res {
                        return Some(LintIgnore { gitignore, path });
                    }
                }
            }
            if current == workspace_root {
                break;
            }
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => break,
            }
        }
        None
    }

    /// Checks if a file path matches any `.lintignore` rule.
    pub fn is_ignored<P: AsRef<Path>>(&self, file_path: P) -> bool {
        let path = file_path.as_ref();
        let rel_path = if let Ok(current) = std::env::current_dir() {
            if let Ok(stripped) = path.strip_prefix(&current) {
                stripped
            } else {
                path
            }
        } else {
            path
        };
        self.gitignore.matched(rel_path, false).is_ignore()
            || self.gitignore.matched(path, false).is_ignore()
    }
}

/// Helper to find workspace root directory by invoking `cargo metadata` or walking up parent directories.
#[allow(clippy::collapsible_if)]
pub fn find_workspace_root_path(start_dir: &Path) -> PathBuf {
    let mut cmd = Command::new("cargo");
    cmd.args(["metadata", "--no-deps", "--format-version", "1"]);
    cmd.current_dir(start_dir);
    if let Ok(output) = cmd.output() {
        if output.status.success() {
            let json_res = serde_json::from_slice::<serde_json::Value>(&output.stdout);
            if let Ok(val) = json_res {
                if let Some(root_str) = val.get("workspace_root").and_then(|v| v.as_str()) {
                    return PathBuf::from(root_str);
                }
            }
        }
    }
    let mut current = start_dir.to_path_buf();
    let mut candidate = current.clone();
    loop {
        let manifest = current.join("Cargo.toml");
        if manifest.exists() {
            candidate = current.clone();
            let read_res = fs::read_to_string(&manifest);
            if let Ok(content) = read_res {
                if content.contains("[workspace]") {
                    return current;
                }
            }
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }
    candidate
}

/// Discovers `budget.toml` by searching `cwd` and walking up to `workspace_root`.
pub fn discover_config_file(cwd: &Path, workspace_root: &Path) -> Option<PathBuf> {
    let mut current = cwd.to_path_buf();
    loop {
        let budget = current.join("budget.toml");
        if budget.exists() {
            return Some(budget);
        }
        if current == workspace_root {
            break;
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }
    None
}

/// Resolves config path based on CLI `--config` option or by walking up from `cwd` to `workspace_root`.
/// An explicit `--config <PATH>` wins and errors if the specified file does not exist.
pub fn resolve_config(config_arg: Option<&str>) -> Result<Option<PathBuf>, String> {
    if let Some(path_str) = config_arg {
        let path = PathBuf::from(path_str);
        if path.exists() {
            Ok(Some(path))
        } else {
            Err(format!("Error: Config file '{}' does not exist", path_str))
        }
    } else {
        let cwd = std::env::current_dir()
            .map_err(|e| format!("Error: Failed to get current directory: {}", e))?;
        let workspace_root = find_workspace_root_path(&cwd);
        Ok(discover_config_file(&cwd, &workspace_root))
    }
}

/// Loads `path` as a validated `BudgetConfig` and formats its `[lints]`
/// entries into `-A`/`-W`/`-D` flags for `DYLINT_RUSTFLAGS`. Validation
/// (unknown lint names, invalid levels) is handled by
/// `BudgetConfig::from_file_validated`, the single canonical config parser.
pub fn parse_budget_config(path: &str) -> Result<Vec<String>, String> {
    let config = BudgetConfig::from_file_validated(Path::new(path), LINT_NAMES)?;
    Ok(config.to_lint_flags())
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
pub fn try_parse_budget_config(path: &str) -> Result<Vec<String>, String> {
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

/// Loads and validates `budget.toml` from `path` into `BudgetConfig` using safe defaults
/// (returning `Ok(None)`) when the file cannot be read or parsed, but propagating
/// validation errors.
pub fn load_budget_config_lenient(path: &Path) -> Result<Option<BudgetConfig>, String> {
    match BudgetConfig::from_file_validated(path, LINT_NAMES) {
        Ok(cfg) => Ok(Some(cfg)),
        Err(e)
            if e.starts_with("Error: Failed to read")
                || e.starts_with("Error: Failed to parse") =>
        {
            eprintln!("warning: {e}\n         continuing with safe defaults (no lint overrides).");
            Ok(None)
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

    if cli.clear_cache {
        let cache_dir = cache::get_cache_dir(None);
        match cache::clear_cache(&cache_dir) {
            Ok(count) => {
                println!(
                    "Cleared {} cached lint result(s) at {}",
                    count,
                    cache_dir.display()
                );
                return;
            }
            Err(e) => {
                eprintln!("{}", e);
                exit(1);
            }
        }
    }

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
    let mut resolved_config_path: Option<PathBuf> = None;
    let mut config_opt: Option<BudgetConfig> = None;

    if cli.fix {
        if let Err(e) = check_git_clean(cli.allow_dirty, cli.allow_staged) {
            eprintln!("{}", e);
            exit(1);
        }
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let workspace_root = find_workspace_root_path(&cwd);
    let lintignore_opt = LintIgnore::discover(&cwd, &workspace_root);

    let resolved_config = match resolve_config(cli.config.as_deref()) {
        Ok(path_opt) => path_opt,
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    };

    if let Some(ref path) = resolved_config {
        resolved_config_path = Some(path.clone());
        if verbose {
            eprintln!("Using config file: {}", path.display());
        } else if !quiet {
            eprintln!("Using config: {}", path.display());
        }
        match load_budget_config_lenient(path) {
            Ok(cfg) => {
                config_opt = cfg;
            }
            Err(e) => {
                eprintln!("{}", e);
                exit(1);
            }
        }
    } else {
        if verbose {
            eprintln!("No budget.toml found, using default lint levels.");
        } else if !quiet && cli.allow.is_empty() && cli.warn.is_empty() && cli.deny.is_empty() {
            eprintln!("Warning: budget.toml not found, using default lint levels.");
        }
    }

    let lint_flags =
        match build_effective_lint_flags(config_opt.as_ref(), &cli.allow, &cli.warn, &cli.deny) {
            Ok(flags) => flags,
            Err(e) => {
                eprintln!("{}", e);
                exit(1);
            }
        };

    let package_args = if !cli.package.is_empty() || cli.workspace {
        let available_packages = if !cli.package.is_empty() {
            match get_workspace_packages(None) {
                Ok(pkgs) => pkgs,
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                }
            }
        } else {
            Vec::new()
        };

        match validate_and_build_package_args(&cli.package, cli.workspace, &available_packages) {
            Ok(args) => args,
            Err(e) => {
                eprintln!("{}", e);
                exit(1);
            }
        }
    } else {
        Vec::new()
    };

    let cache_dir = cache::get_cache_dir(None);
    let cache_key_hash = if !cli.no_cache {
        let source_hash = cache::compute_source_hash(Path::new("."))
            .unwrap_or_else(|_| "unknown-source".to_string());
        let toolchain = cache::get_toolchain_version();
        let key = cache::CacheKey {
            linter_version: env!("CARGO_PKG_VERSION").to_string(),
            toolchain,
            lint_flags: lint_flags.clone(),
            package_args: package_args.clone(),
            output_format: format!("{:?}", cli.format),
            source_hash,
        };
        Some(key.compute_hash())
    } else {
        None
    };

    if let Some(ref key_hash) = cache_key_hash {
        if let Some(cached_entry) = cache::load_cache_entry(&cache_dir, key_hash) {
            if verbose {
                eprintln!("[verbose] Cache hit for key {}", key_hash);
            }
            if !cached_entry.stdout.is_empty() {
                print!("{}", cached_entry.stdout);
            }
            if !cached_entry.stderr.is_empty() {
                eprint!("{}", cached_entry.stderr);
            }
            exit(cached_entry.exit_code);
        } else if verbose {
            eprintln!("[verbose] Cache miss for key {}", key_hash);
        }
    }

    let mut cmd = Command::new("cargo");
    let effective_color = resolve_color_choice(&cli.color);
    if let Some(color_val) = effective_color.as_cargo_arg() {
        cmd.arg("--color");
        cmd.arg(color_val);
        cmd.env("CARGO_TERM_COLOR", color_val);
    }
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

    let mut cargo_args = Vec::new();
    cargo_args.extend(package_args);

    let has_lintignore = lintignore_opt.is_some();
    let has_baseline = cli.baseline.is_some();
    let is_blessing = cli.bless || std::env::var("BLESS").is_ok_and(|v| v == "1" || v == "true");
    let needs_json = cli.format != OutputFormat::Text || has_lintignore || has_baseline || cli.fix;

    if needs_json {
        cargo_args.push("--message-format=json".to_string());
    }

    if !cargo_args.is_empty() {
        cmd.arg("--");
        for arg in cargo_args {
            cmd.arg(arg);
        }
    }

    if verbose {
        if let Some(ref p) = resolved_config_path {
            eprintln!("[verbose] config: {}", p.display());
        } else {
            eprintln!("[verbose] config: (none — using default lint levels)");
        }
        if let Some(ref li) = lintignore_opt {
            eprintln!("[verbose] .lintignore: {}", li.path.display());
        }
        if !rustflags_value.is_empty() {
            eprintln!("[verbose] DYLINT_RUSTFLAGS: {}", rustflags_value);
        } else {
            eprintln!("[verbose] DYLINT_RUSTFLAGS: (empty)");
        }
        eprintln!("[verbose] command: {:?}", cmd);
    }

    if needs_json {
        cmd.stdout(Stdio::piped());
        let mut child = cmd
            .spawn()
            .expect("Failed to execute cargo dylint. Is cargo-dylint installed?");

        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let reader = BufReader::new(stdout);
        let mut highest_exit_code = 0;
        let mut raw_findings: Vec<(LintFinding, String)> = Vec::new();
        let mut machine_fixes: Vec<MachineFix> = Vec::new();
        let mut recorded_stdout = String::new();

        for line_str in reader.lines().map_while(Result::ok) {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line_str) {
                if msg.get("reason").and_then(|r| r.as_str()) == Some("compiler-message") {
                    if let Some(message) = msg.get("message") {
                        if let Some(code) = message.get("code") {
                            if let Some(lint_name) = code.get("code").and_then(|c| c.as_str()) {
                                if LINT_NAMES.contains(&lint_name) {
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

                                    if let Some(ref lintignore) = lintignore_opt {
                                        if !file.is_empty() && lintignore.is_ignored(&file) {
                                            if verbose {
                                                eprintln!(
                                                    "[verbose] Suppressing finding in {} due to .lintignore pattern",
                                                    file
                                                );
                                            }
                                            continue;
                                        }
                                    }

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
                                            }

                                            // Extract MachineApplicable suggestions
                                            if let Some(suggs) = child_item
                                                .get("suggestions")
                                                .and_then(|s| s.as_array())
                                            {
                                                for sug in suggs {
                                                    if sug
                                                        .get("applicability")
                                                        .and_then(|a| a.as_str())
                                                        == Some("MachineApplicable")
                                                    {
                                                        if let Some(parts) = sug
                                                            .get("parts")
                                                            .and_then(|p| p.as_array())
                                                        {
                                                            for part in parts {
                                                                let f = part
                                                                    .get("file_name")
                                                                    .and_then(|f| f.as_str())
                                                                    .unwrap_or("");
                                                                let ls = part
                                                                    .get("line_start")
                                                                    .and_then(|l| l.as_u64())
                                                                    .unwrap_or(0)
                                                                    as usize;
                                                                let cs = part
                                                                    .get("column_start")
                                                                    .and_then(|c| c.as_u64())
                                                                    .unwrap_or(0)
                                                                    as usize;
                                                                let le = part
                                                                    .get("line_end")
                                                                    .and_then(|l| l.as_u64())
                                                                    .unwrap_or(0)
                                                                    as usize;
                                                                let ce = part
                                                                    .get("column_end")
                                                                    .and_then(|c| c.as_u64())
                                                                    .unwrap_or(0)
                                                                    as usize;
                                                                let rep = part
                                                                    .get("snippet")
                                                                    .and_then(|s| s.as_str())
                                                                    .unwrap_or("")
                                                                    .to_string();
                                                                if !f.is_empty() {
                                                                    machine_fixes.push(
                                                                        MachineFix {
                                                                            file_path: f
                                                                                .to_string(),
                                                                            line_start: ls,
                                                                            column_start: cs,
                                                                            line_end: le,
                                                                            column_end: ce,
                                                                            replacement: rep,
                                                                            _lint_name: lint_name
                                                                                .to_string(),
                                                                        },
                                                                    );
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
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

                                    let rendered = message
                                        .get("rendered")
                                        .and_then(|r| r.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    raw_findings.push((finding, rendered));
                                }
                            }
                        }
                    }
                }
            }
        }

        let status = child.wait().expect("Failed to wait on cargo dylint");

        // Execute --fix if requested
        if cli.fix {
            match apply_machine_fixes(&machine_fixes) {
                Ok((applied, files)) => {
                    if applied > 0 && !quiet {
                        eprintln!("Applied {} fix(es) across {} file(s).", applied, files);
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                }
            }
        }

        // Handle --baseline
        let mut final_findings = Vec::new();
        let exit_code;

        if let Some(ref b_path_str) = cli.baseline {
            let b_path = PathBuf::from(b_path_str);
            let mut occ_map: std::collections::HashMap<(String, String, String), usize> =
                std::collections::HashMap::new();

            if is_blessing {
                let mut b_list = Vec::new();
                for (finding, _) in &raw_findings {
                    let rel_file = normalize_file_path(&finding.file);
                    let (snippet, context) = extract_finding_context(
                        &finding.file,
                        finding.span.line_start,
                        finding.span.line_end,
                    );
                    let ctx_hash = compute_context_hash(&finding.name, &rel_file, &context);
                    let key = (finding.name.clone(), rel_file.clone(), ctx_hash.clone());
                    let count = occ_map.entry(key).or_insert(0);
                    *count += 1;
                    b_list.push(BaselineFinding {
                        lint_name: finding.name.clone(),
                        file: rel_file,
                        context_hash: ctx_hash,
                        code_snippet: snippet,
                        occurrence: *count,
                    });
                }
                b_list.sort_by(|a, b| {
                    a.file
                        .cmp(&b.file)
                        .then_with(|| a.lint_name.cmp(&b.lint_name))
                        .then_with(|| a.code_snippet.cmp(&b.code_snippet))
                        .then_with(|| a.occurrence.cmp(&b.occurrence))
                });
                let baseline = Baseline {
                    version: 1,
                    findings: b_list,
                };
                let json_out =
                    serde_json::to_string_pretty(&baseline).expect("Failed to serialize baseline");
                if let Err(e) = fs::write(&b_path, json_out) {
                    eprintln!("Error writing baseline file {}: {}", b_path.display(), e);
                    exit(1);
                }
                if !quiet {
                    eprintln!(
                        "Baseline saved to {} ({} findings)",
                        b_path.display(),
                        baseline.findings.len()
                    );
                }
                final_findings = raw_findings;
                exit_code = 0;
            } else {
                let content = match fs::read_to_string(&b_path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!(
                            "Error: Baseline file not found or unreadable {}: {}",
                            b_path.display(),
                            e
                        );
                        exit(1);
                    }
                };
                let baseline: Baseline = match serde_json::from_str(&content) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("Error parsing baseline JSON {}: {}", b_path.display(), e);
                        exit(1);
                    }
                };

                let mut matched_base = vec![false; baseline.findings.len()];
                let mut suppressed_count = 0;

                for item in raw_findings {
                    let (ref finding, _) = item;
                    let rel_file = normalize_file_path(&finding.file);
                    let (_snippet, context) = extract_finding_context(
                        &finding.file,
                        finding.span.line_start,
                        finding.span.line_end,
                    );
                    let ctx_hash = compute_context_hash(&finding.name, &rel_file, &context);
                    let key = (finding.name.clone(), rel_file.clone(), ctx_hash.clone());
                    let count = occ_map.entry(key).or_insert(0);
                    *count += 1;

                    let mut suppressed = false;
                    for (idx, b_item) in baseline.findings.iter().enumerate() {
                        if !matched_base[idx]
                            && b_item.lint_name == finding.name
                            && b_item.file == rel_file
                            && b_item.context_hash == ctx_hash
                            && b_item.occurrence == *count
                        {
                            matched_base[idx] = true;
                            suppressed = true;
                            suppressed_count += 1;
                            break;
                        }
                    }

                    if !suppressed {
                        final_findings.push(item);
                    }
                }

                for (idx, b_item) in baseline.findings.iter().enumerate() {
                    if !matched_base[idx] && !quiet {
                        eprintln!(
                            "Fixed finding (no longer present): {} in {}",
                            b_item.lint_name, b_item.file
                        );
                    }
                }

                if suppressed_count > 0 && !quiet {
                    eprintln!("Suppressed {} baseline finding(s)", suppressed_count);
                }

                let has_errors = final_findings
                    .iter()
                    .any(|(f, _)| f.level == "error" || f.level == "deny");
                exit_code = if has_errors { 1 } else { 0 };
            }
        } else {
            final_findings = raw_findings;
            exit_code = if !status.success() {
                status.code().unwrap_or(1)
            } else if highest_exit_code != 0 {
                highest_exit_code
            } else {
                0
            };
        }

        // Format final findings
        let mut sarif_findings = Vec::new();
        for (finding, rendered) in &final_findings {
            match cli.format {
                OutputFormat::Text => {
                    if !rendered.is_empty() {
                        eprint!("{}", rendered);
                        recorded_stdout.push_str(rendered);
                    }
                }
                OutputFormat::Json => {
                    if let Ok(json_str) = serde_json::to_string(finding) {
                        println!("{}", json_str);
                        recorded_stdout.push_str(&json_str);
                        recorded_stdout.push('\n');
                    }
                }
                OutputFormat::Github => {
                    let mut buf = Vec::new();
                    if output_formatters::emit_github_annotation(finding, &mut buf).is_ok() {
                        let ann_str = String::from_utf8_lossy(&buf);
                        print!("{}", ann_str);
                        recorded_stdout.push_str(&ann_str);
                    }
                }
                OutputFormat::Sarif => {
                    sarif_findings.push(finding.clone());
                }
            }
        }

        if cli.format == OutputFormat::Sarif {
            let mut buf = Vec::new();
            if output_formatters::emit_sarif(&sarif_findings, &mut buf).is_ok() {
                let sarif_str = String::from_utf8_lossy(&buf);
                print!("{}", sarif_str);
                recorded_stdout.push_str(&sarif_str);
            }
        }

        if let Some(ref key_hash) = cache_key_hash {
            let entry = cache::CacheEntry {
                key: key_hash.clone(),
                exit_code,
                stdout: recorded_stdout,
                stderr: String::new(),
            };
            let _ = cache::save_cache_entry(&cache_dir, key_hash, &entry);
        }

        exit(exit_code);
    } else {
        let output = cmd
            .output()
            .expect("Failed to execute cargo dylint. Is cargo-dylint installed?");

        let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();

        if !stdout_str.is_empty() {
            print!("{}", stdout_str);
        }
        if !stderr_str.is_empty() {
            eprint!("{}", stderr_str);
        }

        let exit_code = output
            .status
            .code()
            .unwrap_or(if output.status.success() { 0 } else { 1 });

        if let Some(ref key_hash) = cache_key_hash {
            let entry = cache::CacheEntry {
                key: key_hash.clone(),
                exit_code,
                stdout: stdout_str,
                stderr: stderr_str,
            };
            let _ = cache::save_cache_entry(&cache_dir, key_hash, &entry);
        }

        exit(exit_code);
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
    fn lint_info_matches_lint_inventory() {
        assert_eq!(
            LINT_INFO.len(),
            LINT_INVENTORY.lints.len(),
            "LINT_INFO and LINT_INVENTORY must have identical number of entries"
        );
        for (info, inv) in LINT_INFO.iter().zip(LINT_INVENTORY.lints.iter()) {
            assert_eq!(info.name, inv.name, "Lint names must match");
            assert_eq!(
                info.level, inv.default_level,
                "Lint default levels must match for {}",
                info.name
            );
            assert_eq!(
                info.description, inv.description,
                "Lint descriptions must match for {}",
                info.name
            );
        }
    }

    #[test]
    fn test_declare_lint_parser_with_comma_in_description() {
        let sample = r#"
        rustc_session::declare_lint! {
            pub TEST_COMMA_LINT,
            Warn,
            "this description has, multiple, commas, and formatting"
        }
        "#;
        let mut depth: u32 = 0;
        let mut end_offset = 0;
        let start = sample.find("declare_lint! {").unwrap();
        let after_start = &sample[start + "declare_lint! {".len()..];
        depth += 1;
        for (i, ch) in after_start.char_indices() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    end_offset = i;
                    break;
                }
            }
        }
        assert!(end_offset > 0);
        let block = &after_start[..end_offset];
        let lines: Vec<&str> = block
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with("#["))
            .collect();
        assert!(lines.len() >= 3);
        let name = lines[0]
            .strip_prefix("pub ")
            .unwrap_or(lines[0])
            .trim_end_matches(',')
            .trim()
            .to_lowercase();
        let level = lines[1].trim_end_matches(',').trim().to_lowercase();
        let raw_desc = lines[2..].join(" ");
        let trimmed_desc = raw_desc.trim().trim_end_matches(',').trim();
        let description = if trimmed_desc.starts_with('"')
            && trimmed_desc.ends_with('"')
            && trimmed_desc.len() >= 2
        {
            trimmed_desc[1..trimmed_desc.len() - 1].to_string()
        } else {
            trimmed_desc.trim_matches('"').to_string()
        };

        assert_eq!(name, "test_comma_lint");
        assert_eq!(level, "warn");
        assert_eq!(
            description,
            "this description has, multiple, commas, and formatting"
        );
    }

    #[test]
    fn lint_registry_json_matches_registered_lints() {
        let registry_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/lints/lint-registry.json"
        );
        let content = std::fs::read_to_string(registry_path)
            .expect("lint-registry.json must exist and be readable");
        let parsed: Vec<serde_json::Value> =
            serde_json::from_str(&content).expect("lint-registry.json must be valid JSON");

        let registered_names = LINT_NAMES
            .iter()
            .map(|s| s.to_string())
            .collect::<std::collections::HashSet<_>>();
        let registry_names = parsed
            .iter()
            .map(|entry| {
                entry["name"]
                    .as_str()
                    .expect("name field must be string")
                    .to_string()
            })
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(
            registered_names, registry_names,
            "lint-registry.json entries must match registered lints exactly"
        );
        assert_eq!(
            parsed.len(),
            registered_names.len(),
            "lint-registry.json must have no duplicate entries"
        );
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

    #[test]
    fn load_budget_config_lenient_handles_valid_missing_and_invalid() {
        let dir = std::env::temp_dir().join("cost_lint_test_load_lenient");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // 1. Missing file -> Ok(None)
        let missing = dir.join("missing_budget.toml");
        let res_missing = load_budget_config_lenient(&missing);
        assert!(res_missing.is_ok());
        assert_eq!(res_missing.unwrap(), None);

        // 2. Unparseable file -> Ok(None)
        let invalid = dir.join("invalid_budget.toml");
        fs::write(&invalid, "invalid toml {{{{").unwrap();
        let res_invalid = load_budget_config_lenient(&invalid);
        assert!(res_invalid.is_ok());
        assert_eq!(res_invalid.unwrap(), None);

        // 3. Valid file -> Ok(Some(BudgetConfig))
        let valid = dir.join("valid_budget.toml");
        fs::write(&valid, "[lints]\nsoroban_storage_in_loop = \"deny\"\n").unwrap();
        let res_valid = load_budget_config_lenient(&valid);
        assert!(res_valid.is_ok());
        let cfg = res_valid.unwrap().expect("should parse config");
        assert_eq!(
            cfg.lints
                .as_ref()
                .unwrap()
                .get("soroban_storage_in_loop")
                .map(|s| s.as_str()),
            Some("deny")
        );

        // 4. Unknown lint level -> Err(...)
        let bad_level = dir.join("bad_level_budget.toml");
        fs::write(&bad_level, "[lints]\nsoroban_storage_in_loop = \"oops\"\n").unwrap();
        let res_bad = load_budget_config_lenient(&bad_level);
        assert!(res_bad.is_err());
        assert!(res_bad.unwrap_err().contains("Unknown lint level"));

        let _ = fs::remove_dir_all(&dir);
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
    fn cli_parses_format_github() {
        let cli = Cli::try_parse_from(["cargo-cost-lint", "--format", "github"])
            .expect("parsing should succeed");
        assert_eq!(cli.format, OutputFormat::Github);
    }

    #[test]
    fn cli_parses_format_sarif() {
        let cli = Cli::try_parse_from(["cargo-cost-lint", "--format", "sarif"])
            .expect("parsing should succeed");
        assert_eq!(cli.format, OutputFormat::Sarif);
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

    #[test]
    fn cli_parses_allow_warn_deny_flags() {
        let cli = Cli::try_parse_from([
            "cargo-cost-lint",
            "--allow",
            "redundant_env_clone",
            "--warn",
            "map_insert_in_loop",
            "--deny",
            "soroban_storage_in_loop",
        ])
        .expect("parsing should succeed");
        assert_eq!(cli.allow, vec!["redundant_env_clone"]);
        assert_eq!(cli.warn, vec!["map_insert_in_loop"]);
        assert_eq!(cli.deny, vec!["soroban_storage_in_loop"]);
    }

    #[test]
    fn cli_parses_short_override_flags() {
        let cli = Cli::try_parse_from([
            "cargo-cost-lint",
            "-A",
            "redundant_env_clone",
            "-W",
            "map_insert_in_loop",
            "-D",
            "soroban_storage_in_loop",
        ])
        .expect("parsing should succeed");
        assert_eq!(cli.allow, vec!["redundant_env_clone"]);
        assert_eq!(cli.warn, vec!["map_insert_in_loop"]);
        assert_eq!(cli.deny, vec!["soroban_storage_in_loop"]);
    }

    #[test]
    fn cli_parses_repeatable_override_flags() {
        let cli = Cli::try_parse_from([
            "cargo-cost-lint",
            "--allow",
            "redundant_env_clone",
            "--allow",
            "symbol_new_for_short_literal",
            "--deny",
            "soroban_storage_in_loop",
            "--deny",
            "map_insert_in_loop",
        ])
        .expect("parsing should succeed");
        assert_eq!(
            cli.allow,
            vec!["redundant_env_clone", "symbol_new_for_short_literal"]
        );
        assert_eq!(
            cli.deny,
            vec!["soroban_storage_in_loop", "map_insert_in_loop"]
        );
    }

    #[test]
    fn test_effective_flags_cli_only() {
        let allow = vec!["redundant_env_clone".to_string()];
        let warn = vec!["map_insert_in_loop".to_string()];
        let deny = vec!["soroban_storage_in_loop".to_string()];

        let flags = build_effective_lint_flags(None, &allow, &warn, &deny).unwrap();
        assert_eq!(
            flags,
            vec![
                "-W map_insert_in_loop",
                "-A redundant_env_clone",
                "-D soroban_storage_in_loop",
            ]
        );
    }

    #[test]
    fn test_effective_flags_precedence_over_budget_toml() {
        // budget.toml sets soroban_storage_in_loop to "warn" and redundant_env_clone to "deny"
        let mut lints = std::collections::HashMap::new();
        lints.insert("soroban_storage_in_loop".to_string(), "warn".to_string());
        lints.insert("redundant_env_clone".to_string(), "deny".to_string());
        let config = BudgetConfig { lints: Some(lints) };

        // CLI overrides soroban_storage_in_loop to "deny" and redundant_env_clone to "allow"
        let allow = vec!["redundant_env_clone".to_string()];
        let warn = vec![];
        let deny = vec!["soroban_storage_in_loop".to_string()];

        let flags = build_effective_lint_flags(Some(&config), &allow, &warn, &deny).unwrap();
        assert_eq!(
            flags,
            vec!["-A redundant_env_clone", "-D soroban_storage_in_loop",]
        );
    }

    #[test]
    fn test_effective_flags_preserves_unoverridden_budget_toml_lints() {
        let mut lints = std::collections::HashMap::new();
        lints.insert("soroban_storage_in_loop".to_string(), "warn".to_string());
        lints.insert("redundant_env_clone".to_string(), "warn".to_string());
        let config = BudgetConfig { lints: Some(lints) };

        // Only override soroban_storage_in_loop on CLI; redundant_env_clone should remain "warn"
        let allow = vec![];
        let warn = vec![];
        let deny = vec!["soroban_storage_in_loop".to_string()];

        let flags = build_effective_lint_flags(Some(&config), &allow, &warn, &deny).unwrap();
        assert_eq!(
            flags,
            vec!["-W redundant_env_clone", "-D soroban_storage_in_loop",]
        );
    }

    #[test]
    fn test_effective_flags_rejects_unknown_lint_name_cli() {
        let allow = vec!["invalid_lint_name".to_string()];
        let result = build_effective_lint_flags(None, &allow, &[], &[]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Unknown lint name 'invalid_lint_name'"));
        assert!(err.contains("Valid lints are:"));
    }

    #[test]
    fn test_effective_flags_rejects_conflicting_cli_flags() {
        let allow = vec!["soroban_storage_in_loop".to_string()];
        let deny = vec!["soroban_storage_in_loop".to_string()];
        let result = build_effective_lint_flags(None, &allow, &[], &deny);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Conflicting lint levels specified for 'soroban_storage_in_loop'"));
    }

    #[test]
    fn test_effective_flags_allows_duplicate_same_level_cli_flags() {
        let allow = vec![
            "redundant_env_clone".to_string(),
            "redundant_env_clone".to_string(),
        ];
        let result = build_effective_lint_flags(None, &allow, &[], &[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["-A redundant_env_clone"]);
    }

    #[test]
    fn cli_parses_package_flag() {
        let cli = Cli::try_parse_from(["cargo-cost-lint", "--package", "my-contract"])
            .expect("parsing should succeed");
        assert_eq!(cli.package, vec!["my-contract"]);
        assert!(!cli.workspace);

        let cli_short = Cli::try_parse_from(["cargo-cost-lint", "-p", "my-contract"])
            .expect("parsing should succeed");
        assert_eq!(cli_short.package, vec!["my-contract"]);
    }

    #[test]
    fn cli_parses_repeatable_package_flags() {
        let cli = Cli::try_parse_from([
            "cargo-cost-lint",
            "-p",
            "contract-a",
            "--package",
            "contract-b",
        ])
        .expect("parsing should succeed");
        assert_eq!(cli.package, vec!["contract-a", "contract-b"]);
    }

    #[test]
    fn cli_parses_workspace_flag() {
        let cli = Cli::try_parse_from(["cargo-cost-lint", "--workspace"])
            .expect("parsing should succeed");
        assert!(cli.workspace);
        assert!(cli.package.is_empty());
    }

    #[test]
    fn test_validate_and_build_package_args_default() {
        let pkgs = vec![];
        let workspace = false;
        let available = vec!["contract-a".to_string(), "contract-b".to_string()];
        let args = validate_and_build_package_args(&pkgs, workspace, &available).unwrap();
        assert!(args.is_empty());
    }

    #[test]
    fn test_validate_and_build_package_args_single_package() {
        let pkgs = vec!["contract-a".to_string()];
        let workspace = false;
        let available = vec!["contract-a".to_string(), "contract-b".to_string()];
        let args = validate_and_build_package_args(&pkgs, workspace, &available).unwrap();
        assert_eq!(args, vec!["--package", "contract-a"]);
    }

    #[test]
    fn test_validate_and_build_package_args_multiple_packages() {
        let pkgs = vec!["contract-a".to_string(), "contract-b".to_string()];
        let workspace = false;
        let available = vec!["contract-a".to_string(), "contract-b".to_string()];
        let args = validate_and_build_package_args(&pkgs, workspace, &available).unwrap();
        assert_eq!(
            args,
            vec!["--package", "contract-a", "--package", "contract-b"]
        );
    }

    #[test]
    fn test_validate_and_build_package_args_workspace() {
        let pkgs = vec![];
        let workspace = true;
        let available = vec!["contract-a".to_string(), "contract-b".to_string()];
        let args = validate_and_build_package_args(&pkgs, workspace, &available).unwrap();
        assert_eq!(args, vec!["--workspace"]);
    }

    #[test]
    fn test_validate_and_build_package_args_unknown_package() {
        let pkgs = vec!["nonexistent-contract".to_string()];
        let workspace = false;
        let available = vec!["contract-a".to_string(), "contract-b".to_string()];
        let result = validate_and_build_package_args(&pkgs, workspace, &available);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Package 'nonexistent-contract' not found in workspace"));
        assert!(err.contains("Valid workspace members are: contract-a, contract-b"));
    }

    #[test]
    fn test_validate_and_build_package_args_conflicting_workspace_and_package() {
        let pkgs = vec!["contract-a".to_string()];
        let workspace = true;
        let available = vec!["contract-a".to_string(), "contract-b".to_string()];
        let result = validate_and_build_package_args(&pkgs, workspace, &available);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("cannot be used with '--package <SPEC>'"));
    }

    #[test]
    fn test_parse_workspace_members_from_metadata() {
        let sample_json = r#"{
            "packages": [
                {"name": "contract-a", "id": "path+file:///crates/contract-a#0.1.0"},
                {"name": "contract-b", "id": "path+file:///crates/contract-b#0.1.0"},
                {"name": "dep-c", "id": "registry+https://github.com/rust-lang/crates.io-index#0.1.0"}
            ],
            "workspace_members": [
                "path+file:///crates/contract-a#0.1.0",
                "path+file:///crates/contract-b#0.1.0"
            ]
        }"#;

        let members = parse_workspace_members_from_metadata(sample_json.as_bytes()).unwrap();
        assert_eq!(members, vec!["contract-a", "contract-b"]);
    }

    #[test]
    fn cli_parses_no_cache_flag() {
        let cli =
            Cli::try_parse_from(["cargo-cost-lint", "--no-cache"]).expect("parsing should succeed");
        assert!(cli.no_cache);
        assert!(!cli.clear_cache);
    }

    #[test]
    fn cli_parses_clear_cache_flag() {
        let cli = Cli::try_parse_from(["cargo-cost-lint", "--clear-cache"])
            .expect("parsing should succeed");
        assert!(cli.clear_cache);
        assert!(!cli.no_cache);
    }

    // --- Colour / NO_COLOR tests (issue #420) ---

    #[test]
    fn cli_color_default_is_auto() {
        let cli = Cli::try_parse_from(["cargo-cost-lint"]).expect("parsing should succeed");
        assert_eq!(cli.color, ColorChoice::Auto);
    }

    #[test]
    fn cli_parses_color_always() {
        let cli = Cli::try_parse_from(["cargo-cost-lint", "--color", "always"])
            .expect("parsing should succeed");
        assert_eq!(cli.color, ColorChoice::Always);
    }

    #[test]
    fn cli_parses_color_never() {
        let cli = Cli::try_parse_from(["cargo-cost-lint", "--color", "never"])
            .expect("parsing should succeed");
        assert_eq!(cli.color, ColorChoice::Never);
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn resolve_color_explicit_always_overrides_no_color() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("NO_COLOR", "1") };
        let resolved = resolve_color_choice(&ColorChoice::Always);
        assert_eq!(resolved, ColorChoice::Always);
        unsafe { std::env::remove_var("NO_COLOR") };
    }

    #[test]
    fn resolve_color_explicit_never_overrides_no_color() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("NO_COLOR", "1") };
        let resolved = resolve_color_choice(&ColorChoice::Never);
        assert_eq!(resolved, ColorChoice::Never);
        unsafe { std::env::remove_var("NO_COLOR") };
    }

    #[test]
    fn resolve_color_auto_no_color_set_resolves_to_never() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("NO_COLOR", "1") };
        let resolved = resolve_color_choice(&ColorChoice::Auto);
        assert_eq!(resolved, ColorChoice::Never);
        unsafe { std::env::remove_var("NO_COLOR") };
    }

    #[test]
    fn resolve_color_auto_no_color_empty_resolves_to_auto() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("NO_COLOR", "") };
        let resolved = resolve_color_choice(&ColorChoice::Auto);
        assert_eq!(resolved, ColorChoice::Auto);
        unsafe { std::env::remove_var("NO_COLOR") };
    }

    #[test]
    fn resolve_color_auto_no_color_unset_resolves_to_auto() {
        // This test only makes sense when NO_COLOR is not already set in
        // the environment.  On some CI runners (notably Windows) the
        // variable is injected and cannot be reliably removed, so we
        // skip rather than produce a false failure.
        if std::env::var("NO_COLOR").is_ok() {
            eprintln!(
                "skipping resolve_color_auto_no_color_unset_resolves_to_auto: \
                 NO_COLOR is set in the environment"
            );
            return;
        }
        let resolved = resolve_color_choice(&ColorChoice::Auto);
        assert_eq!(resolved, ColorChoice::Auto);
    }

    #[test]
    fn color_choice_as_cargo_arg_auto_returns_none() {
        assert_eq!(ColorChoice::Auto.as_cargo_arg(), None);
    }

    #[test]
    fn color_choice_as_cargo_arg_always_returns_always() {
        assert_eq!(ColorChoice::Always.as_cargo_arg(), Some("always"));
    }

    #[test]
    fn color_choice_as_cargo_arg_never_returns_never() {
        assert_eq!(ColorChoice::Never.as_cargo_arg(), Some("never"));
    }

    #[test]
    fn test_discover_config_in_cwd() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = temp.path();
        let budget_path = cwd.join("budget.toml");
        std::fs::write(&budget_path, "[lints]\n").unwrap();

        let found = discover_config_file(cwd, cwd);
        assert_eq!(found, Some(budget_path));
    }

    #[test]
    fn test_discover_config_in_ancestor() {
        let temp = tempfile::tempdir().unwrap();
        let workspace_root = temp.path();
        let member_dir = workspace_root.join("member");
        std::fs::create_dir_all(&member_dir).unwrap();

        let budget_path = workspace_root.join("budget.toml");
        std::fs::write(&budget_path, "[lints]\n").unwrap();

        let found = discover_config_file(&member_dir, workspace_root);
        assert_eq!(found, Some(budget_path));
    }

    #[test]
    fn test_discover_config_none_found() {
        let temp = tempfile::tempdir().unwrap();
        let workspace_root = temp.path();
        let member_dir = workspace_root.join("member");
        std::fs::create_dir_all(&member_dir).unwrap();

        let found = discover_config_file(&member_dir, workspace_root);
        assert_eq!(found, None);
    }

    #[test]
    fn test_discover_config_stops_at_workspace_root() {
        let temp = tempfile::tempdir().unwrap();
        let parent_dir = temp.path();
        let workspace_root = parent_dir.join("workspace");
        let member_dir = workspace_root.join("member");
        std::fs::create_dir_all(&member_dir).unwrap();

        // Create budget.toml in parent_dir (OUTSIDE workspace)
        std::fs::write(parent_dir.join("budget.toml"), "[lints]\n").unwrap();

        let found = discover_config_file(&member_dir, &workspace_root);
        assert_eq!(found, None);
    }

    #[test]
    fn test_resolve_config_explicit_override_and_missing_error() {
        let temp = tempfile::tempdir().unwrap();
        let valid_config = temp.path().join("my_budget.toml");
        std::fs::write(&valid_config, "[lints]\n").unwrap();

        let res_ok = resolve_config(valid_config.to_str());
        assert!(res_ok.is_ok());
        assert_eq!(res_ok.unwrap(), Some(valid_config));

        let res_err = resolve_config(Some("/nonexistent/file/path/budget.toml"));
        assert!(res_err.is_err());
        assert!(res_err.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_lintignore_matching() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let lintignore_path = root.join(".lintignore");
        std::fs::write(
            &lintignore_path,
            "src/generated/*.rs\nsrc/legacy_batch.rs\n*.tmp.rs\n",
        )
        .unwrap();

        let lintignore = LintIgnore::discover(root, root).expect(".lintignore should be loaded");
        assert!(lintignore.is_ignored(root.join("src/generated/foo.rs")));
        assert!(lintignore.is_ignored(root.join("src/legacy_batch.rs")));
        assert!(lintignore.is_ignored(root.join("test.tmp.rs")));

        assert!(!lintignore.is_ignored(root.join("src/main.rs")));
        assert!(!lintignore.is_ignored(root.join("src/lib.rs")));
    }
}
