use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum Error {
    Io(std::io::Error),
    MissingEnv,
    Parse(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::MissingEnv => write!(
                f,
                "OUT_DIR or CARGO_MANIFEST_DIR environment variable not set"
            ),
            Error::Parse(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

type Result<T> = std::result::Result<T, Error>;

/// A single lint's metadata parsed from `declare_lint!`.
struct LintMeta {
    name: String,        // lowercase snake_case, e.g. "soroban_storage_in_loop"
    level: String,       // lowercase level, e.g. "warn"
    description: String, // one-line description from the macro
}

fn rust_string(value: &str) -> String {
    format!("{:?}", value)
}

/// Parse lint names from every registration site in `lib.rs`, returning
/// lowercase names in registration order.
///
/// `lib.rs` names its lints twice: once in the `dylint_lint_impl! { ..., [...] }`
/// invocation (the list `DEVELOPING_LINTS.md` tells authors to edit) and once in
/// the legacy `lint_store.register_lints(&[...])` call, which the in-tree tests
/// parse instead. Each list has its own parser, so both run whenever their
/// pattern is present and their results are required to agree — otherwise the
/// two parsers would silently describe two different lint sets. When both are
/// present the `dylint_lint_impl!` order wins, matching the documented source of
/// truth.
fn parse_register_lints(content: &str) -> Result<Vec<String>> {
    let dylint = parse_dylint_impl(content)?;
    let legacy = parse_legacy_register_lints(content)?;

    match (dylint, legacy) {
        (Some(dylint_names), Some(legacy_names)) => {
            if dylint_names != legacy_names {
                let dylint_set: HashSet<&str> = dylint_names.iter().map(String::as_str).collect();
                let legacy_set: HashSet<&str> = legacy_names.iter().map(String::as_str).collect();
                let only_dylint = sorted_diff(&dylint_set, &legacy_set);
                let only_legacy = sorted_diff(&legacy_set, &dylint_set);
                if only_dylint.is_empty() && only_legacy.is_empty() {
                    return Err(Error::Parse(
                        "lib.rs lists the same lints in different orders in `dylint_lint_impl!` \
                         and `lint_store.register_lints`; keep the two lists identical"
                            .into(),
                    ));
                }
                return Err(Error::Parse(format!(
                    "lib.rs's `dylint_lint_impl!` and `lint_store.register_lints` lists disagree. \
                     Only in `dylint_lint_impl!`: [{}]. Only in `register_lints`: [{}]. \
                     Keep the two lists identical",
                    only_dylint.join(", "),
                    only_legacy.join(", ")
                )));
            }
            Ok(dylint_names)
        }
        (Some(dylint_names), None) => Ok(dylint_names),
        (None, Some(legacy_names)) => Ok(legacy_names),
        (None, None) => Err(Error::Parse(
            "Could not find register_lints or dylint_lint_impl in lib.rs".into(),
        )),
    }
}

/// Members of `a` that are not in `b`, sorted so the error message is stable.
fn sorted_diff<'a>(a: &HashSet<&'a str>, b: &HashSet<&'a str>) -> Vec<&'a str> {
    let mut only: Vec<&str> = a.difference(b).copied().collect();
    only.sort_unstable();
    only
}

/// Renders lint names as `"a", "b"` so build errors read like the source.
fn join_quoted(names: &[&str]) -> String {
    names
        .iter()
        .map(|name| format!("\"{}\"", name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Parse the lint list out of the legacy `lint_store.register_lints(&[...])`
/// call, if `lib.rs` has one.
fn parse_legacy_register_lints(content: &str) -> Result<Option<Vec<String>>> {
    let start_marker = "lint_store.register_lints(&[";
    let start = match content.find(start_marker) {
        Some(start) => start,
        None => return Ok(None),
    };
    let content_after = &content[start..];
    let end = content_after
        .find("]);")
        .ok_or_else(|| Error::Parse("Could not find end of register_lints".into()))?;

    let list_str = &content_after[start_marker.len()..end];

    let mut names = Vec::new();
    for line in list_str.lines() {
        let trimmed = line.trim().trim_end_matches(',');
        if !trimmed.is_empty() && !trimmed.starts_with("//") {
            names.push(trimmed.to_lowercase());
        }
    }
    Ok(Some(names))
}

/// Parse lint names from the `dylint_lint_impl!` macro invocation, if `lib.rs`
/// has one.
fn parse_dylint_impl(content: &str) -> Result<Option<Vec<String>>> {
    let marker = "dylint_lint_impl!";
    let start = match content.find(marker) {
        Some(start) => start,
        None => return Ok(None),
    };
    let after = &content[start..];
    // The lint list is the macro's second argument, so the first bracket pair
    // opened after the invocation is the one to read.
    let open = after
        .find('[')
        .ok_or_else(|| Error::Parse("dylint_lint_impl! has no lint list".into()))?;
    let close = after[open..]
        .find(']')
        .ok_or_else(|| Error::Parse("dylint_lint_impl! lint list is not closed".into()))?
        + open;
    let list_str = &after[open + 1..close];

    let mut names = Vec::new();
    for line in list_str.lines() {
        let trimmed = line.trim().trim_end_matches(',');
        if !trimmed.is_empty() && !trimmed.starts_with("//") {
            names.push(trimmed.to_lowercase());
        }
    }
    Ok(Some(names))
}

/// Parse `declare_lint! { ... }` blocks to extract each lint's name, default
/// level, and one-line description.
///
/// The block is delimited by [`matching_delimiter`], so a `}` inside a string
/// literal or a comment does not end it early — only the brace that balances
/// the opening one does.
///
/// Returns metadata for lints in the order they appear in source.
fn parse_declare_lints(content: &str) -> Result<Vec<LintMeta>> {
    const MARKER: &str = "declare_lint! {";
    let mut results = Vec::new();
    let mut search_from = 0;

    while let Some(rel_start) = content[search_from..].find(MARKER) {
        let absolute_start = search_from + rel_start;
        let after_start = &content[absolute_start + MARKER.len()..];
        let start_line = content[..absolute_start].lines().count() + 1;

        // The `{` of `declare_lint! {`, and the `}` that balances it.
        let open = absolute_start + MARKER.len() - 1;
        let close = matching_delimiter(content, open).map_err(|e| {
            Error::Parse(format!(
                "unclosed declare_lint! block starting at line {}: {}",
                start_line, e
            ))
        })?;

        let end_idx = close - (absolute_start + MARKER.len());
        let block = &after_start[..end_idx];

        // Extract non-comment, non-empty lines from the block body. Block
        // comments are dropped too, now that the scanner looks inside them.
        let lines: Vec<&str> = block
            .lines()
            .map(str::trim)
            .filter(|l| {
                !l.is_empty()
                    && !l.starts_with("//")
                    && !l.starts_with("/*")
                    && !l.starts_with('*')
                    && !l.starts_with("#[")
            })
            .collect();

        if lines.len() < 3 {
            return Err(Error::Parse(format!(
                "declare_lint! block at line {} has fewer than 3 payload lines (expected name, level, description)",
                start_line
            )));
        }

        // Line 0: "pub LINT_NAME," -> "lint_name"
        let raw_name = lines[0]
            .trim_start_matches("pub ")
            .trim_end_matches(',')
            .trim();
        if raw_name.is_empty() {
            return Err(Error::Parse(format!(
                "declare_lint! block at line {} has empty lint name",
                start_line
            )));
        }
        let name = raw_name.to_lowercase();

        // Line 1: "Warn," -> "warn"
        let raw_level = lines[1].trim_end_matches(',').trim();
        if raw_level.is_empty() {
            return Err(Error::Parse(format!(
                "declare_lint! block for '{}' at line {} has empty lint level",
                name, start_line
            )));
        }
        let level = raw_level.to_lowercase();

        // Line 2+: description (join remaining lines if multiline, trim quotes and commas)
        let raw_description = lines[2..].join(" ");
        let trimmed_desc = raw_description.trim().trim_end_matches(',');
        let description = if trimmed_desc.starts_with('"')
            && trimmed_desc.ends_with('"')
            && trimmed_desc.len() >= 2
        {
            trimmed_desc[1..trimmed_desc.len() - 1].to_string()
        } else {
            trimmed_desc.trim_matches('"').to_string()
        };

        if description.is_empty() {
            return Err(Error::Parse(format!(
                "declare_lint! block for '{}' at line {} has empty description",
                name, start_line
            )));
        }

        results.push(LintMeta {
            name,
            level,
            description,
        });

        search_from = close + 1;
    }

    Ok(results)
}

/// A `LINT_METADATA` row: the fields `declare_lint!` cannot express.
///
/// `name` and `description` are deliberately *not* stored here — they are read
/// out of the row only to check them against the `declare_lint!` block for the
/// same lint, so the two views of `lib.rs` cannot drift apart.
struct RegistryRow {
    category: String,
    description: String,
}

/// Parse the `LINT_METADATA` registry into a lowercase-name → row map.
///
/// This is the third and final view of `lib.rs` that build.rs needs. It is
/// keyed the same way as the `declare_lint!` blocks, so `run()` can require the
/// three views to agree instead of letting a lint quietly lose its category.
fn parse_lint_metadata(content: &str) -> Result<HashMap<String, RegistryRow>> {
    const MARKER: &str = "pub const LINT_METADATA";
    let marker_start = content
        .find(MARKER)
        .ok_or_else(|| Error::Parse("lib.rs has no `pub const LINT_METADATA` registry".into()))?;

    let rel = content[marker_start..].find("= &[").ok_or_else(|| {
        Error::Parse("`LINT_METADATA` in lib.rs is not a slice literal (`= &[ ... ]`)".into())
    })?;
    // Index of the `[` that opens the slice literal.
    let open = marker_start + rel + "= &[".len() - 1;
    let close = matching_delimiter(content, open).map_err(Error::Parse)?;
    let body = &content[open + 1..close];

    let mut rows = HashMap::new();
    for entry in body.split("LintMeta {").skip(1) {
        let mut name = None;
        let mut category = None;
        let mut description = None;
        for line in entry.lines() {
            let line = line.trim().trim_end_matches(',');
            if let Some(value) = line.strip_prefix("name:") {
                name = Some(value.trim().trim_matches('"').to_string());
            } else if let Some(value) = line.strip_prefix("category:") {
                let value = value.trim();
                // `LintCategory::Compute` -> `Compute`
                category = Some(value.rsplit("::").next().unwrap_or(value).to_string());
            } else if let Some(value) = line.strip_prefix("description:") {
                description = Some(value.trim().trim_matches('"').to_string());
            }
        }
        let (name, category, description) = match (name, category, description) {
            (Some(name), Some(category), Some(description)) => (name, category, description),
            _ => {
                return Err(Error::Parse(format!(
                    "Could not parse a LINT_METADATA entry (expected `name:`, `category:` and \
                     `description:` fields): {}",
                    entry.trim()
                )));
            }
        };
        if name.is_empty() || category.is_empty() || description.is_empty() {
            return Err(Error::Parse(format!(
                "LINT_METADATA entry has an empty name, category or description: {}",
                entry.trim()
            )));
        }
        let row = RegistryRow {
            category,
            description,
        };
        if rows.insert(name.to_lowercase(), row).is_some() {
            return Err(Error::Parse(format!(
                "duplicate LINT_METADATA row for lint '{}'",
                name
            )));
        }
    }

    if rows.is_empty() {
        return Err(Error::Parse(
            "LINT_METADATA in lib.rs contains no entries".into(),
        ));
    }
    Ok(rows)
}

/// Index of the delimiter closing the one at `open`.
///
/// `open` must point at an opening `(`, `[` or `{`; the result is the index of
/// the delimiter that balances it. Delimiters inside string literals, raw
/// strings, char literals and comments are skipped, so neither a `}` in a lint
/// description nor a `]` in a comment can end the scan early.
fn matching_delimiter(content: &str, open: usize) -> std::result::Result<usize, String> {
    let Some(from_open) = content.get(open..) else {
        return Err(format!("byte offset {} is not a character boundary", open));
    };
    let Some(first) = from_open.chars().next() else {
        return Err(format!(
            "no delimiter at byte {}: the source ends first",
            open
        ));
    };
    let closer = match first {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        other => {
            return Err(format!(
                "expected `(`, `[` or `{{` at byte {}, found {:?}",
                open, other
            ));
        }
    };

    let mut depth: u32 = 0;
    let mut i = open;
    while i < content.len() {
        // Every advance below keeps `i` on a character boundary.
        let ch = content[i..]
            .chars()
            .next()
            .expect("i is a character boundary");
        let width = ch.len_utf8();
        let rest = &content[i + width..];

        // Comments carry no structure.
        if ch == '/' && rest.starts_with('/') {
            i = match rest.find('\n') {
                Some(newline) => i + width + newline,
                None => content.len(),
            };
            continue;
        }
        if ch == '/' && rest.starts_with('*') {
            let mut nested: u32 = 1;
            i += width + 1; // past the opening `/*`
            while nested > 0 {
                let Some(next) = content[i..].chars().next() else {
                    return Err(format!("unterminated block comment before byte {}", open));
                };
                let next_width = next.len_utf8();
                let next_rest = &content[i + next_width..];
                if next == '*' && next_rest.starts_with('/') {
                    nested -= 1;
                    i += next_width + 1;
                } else if next == '/' && next_rest.starts_with('*') {
                    nested += 1;
                    i += next_width + 1;
                } else {
                    i += next_width;
                }
            }
            continue;
        }

        // Quoted literals hide their contents from the scan.
        if ch == '"' {
            i = skip_quoted(content, i)?;
            continue;
        }
        if ch == '\''
            && let Some(end) = char_literal_end(content, i)
        {
            i = end;
            continue;
        }
        if ch == 'r'
            && let Some(end) = raw_string_end(content, i)
        {
            i = end;
            continue;
        }

        if ch == closer {
            depth = depth
                .checked_sub(1)
                .ok_or_else(|| format!("unmatched {:?} at byte {}", closer, i))?;
            if depth == 0 {
                return Ok(i);
            }
        } else if ch == first {
            depth += 1;
        }

        i += width;
    }

    Err(format!(
        "no {:?} closes the {:?} at byte {}",
        closer, first, open
    ))
}

/// Index just past the closing quote of the `"..."` literal at `start`,
/// honouring backslash escapes.
fn skip_quoted(content: &str, start: usize) -> std::result::Result<usize, String> {
    let mut i = start + 1; // past the opening quote
    while i < content.len() {
        let Some(ch) = content[i..].chars().next() else {
            break;
        };
        let width = ch.len_utf8();
        match ch {
            '\\' => {
                i += 1;
                if let Some(escaped) = content[i..].chars().next() {
                    i += escaped.len_utf8();
                }
            }
            '"' => return Ok(i + width),
            _ => i += width,
        }
    }
    Err(format!("unterminated string literal at byte {}", start))
}

/// If `content[start..]` opens a char literal (`'a'`, `'\n'`, `'{'`, ...), the
/// index just past its closing quote; `None` for a lifetime such as `'a`.
fn char_literal_end(content: &str, start: usize) -> Option<usize> {
    let mut i = start + 1; // past the opening quote
    if content.get(i..)?.starts_with('\\') {
        // An escape sequence, then the closing quote.
        i += 1;
        i += content.get(i..)?.chars().next()?.len_utf8();
        while i < content.len() {
            let ch = content.get(i..)?.chars().next()?;
            i += ch.len_utf8();
            if ch == '\'' {
                return Some(i);
            }
            if ch == '\n' {
                return None;
            }
        }
        return None;
    }
    let first = content.get(i..)?.chars().next()?;
    i += first.len_utf8();
    content.get(i..)?.starts_with('\'').then_some(i + 1)
}

/// If `content[start..]` opens a raw string literal (`r"..."`, `r#"..."#`,
/// ...), the index just past its closing delimiter; `None` otherwise.
fn raw_string_end(content: &str, start: usize) -> Option<usize> {
    let mut i = start + 1; // past the `r`
    let mut hashes = 0usize;
    while content.get(i..)?.starts_with('#') {
        i += 1;
        hashes += 1;
    }
    if !content.get(i..)?.starts_with('"') {
        return None;
    }
    i += 1; // past the opening quote
    if hashes == 0 {
        return content[i..].find('"').map(|rel| i + rel + 1);
    }
    let terminator = format!("\"{}", "#".repeat(hashes));
    content[i..]
        .find(&terminator)
        .map(|rel| i + rel + terminator.len())
}

/// Wraps `s` in the shortest raw string literal (`r"..."`, `r#"..."#`, ...)
/// that can hold it verbatim.
fn raw_string_literal(s: &str) -> String {
    // Find the smallest n such that " followed by n # signs does not appear
    // in the string, so r###"..."### is a valid raw string literal.
    let mut hashes: usize = 0;
    loop {
        let needle: String = format!("\"{}", "#".repeat(hashes));
        if s.contains(&needle) {
            hashes += 1;
        } else {
            break;
        }
    }
    let hash_str = "#".repeat(hashes);
    format!("r{hash_str}\"{}\"{hash_str}", s, hash_str = hash_str)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

/// Parse the pinned nightly channel from a `rust-toolchain` TOML file.
///
/// The file has the shape:
/// ```toml
/// [toolchain]
/// channel = "nightly-YYYY-MM-DD"
/// ```
///
/// We extract the `channel` value so it can be embedded in the binary
/// at build time.
fn parse_toolchain_channel(toolchain_path: &Path) -> Result<String> {
    let content = fs::read_to_string(toolchain_path).map_err(|e| {
        Error::Parse(format!(
            "Failed to read {}: {}",
            toolchain_path.display(),
            e
        ))
    })?;

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("channel") {
            let value = value.trim();
            // channel = "nightly-2026-04-16"
            if let Some(value) = value.strip_prefix('=') {
                let value = value.trim();
                if let Some(value) = value.strip_prefix('"')
                    && let Some(value) = value.strip_suffix('"')
                {
                    return Ok(value.to_string());
                }
            }
        }
    }

    Err(Error::Parse(format!(
        "Could not find 'channel' in {}",
        toolchain_path.display()
    )))
}

/// The cargo-dylint version constraint this tool expects.
/// Updated manually when the minimum supported version changes.
const DYLINT_VERSION_CONSTRAINT: &str = "^6.0.1";

/// Workspace-relative files that repeat the `cargo-dylint` install pin as a
/// `--version "<constraint>"` argument to `cargo install`.
///
/// The pinned nightly has a drift guard (`lint.yml` compares `rust-toolchain`
/// against the schedule); this constant had none. Bumping an install line in
/// only one of these files while leaving `DYLINT_VERSION_CONSTRAINT` alone
/// makes `cargo cost-lint --version` advertise a constraint the CI using it
/// does not install, so `check_dylint_version_pins` fails the build when the
/// two sides drift apart.
///
/// `soroban_cost_lints/Cargo.toml`'s `dylint` dev-dependency table is
/// deliberately not a pin site, and `CONTRIBUTING.md`'s prose mentions are
/// not gated either; this gate covers only the executable install lines.
const DYLINT_PIN_SITES: &[&str] = &[
    ".github/workflows/lint.yml",
    ".github/workflows/nightly-bump.yml",
    "action.yml",
    "Dockerfile",
    "templates/github-action.yml",
];

/// Collect every `--version "<constraint>"` argument in `content` as
/// (1-based line, constraint) pairs. Lines with an unterminated quote are
/// ignored.
fn extract_version_constraints(content: &str) -> Vec<(usize, String)> {
    const FLAG: &str = "--version \"";
    let mut found = Vec::new();
    for (lineno, line) in content.lines().enumerate() {
        let mut offset = 0;
        while let Some(rel) = line[offset..].find(FLAG) {
            let start = offset + rel + FLAG.len();
            match line[start..].find('"') {
                Some(end) => {
                    found.push((lineno + 1, line[start..start + end].to_string()));
                    offset = start + end;
                }
                None => break,
            }
        }
    }
    found
}

/// Read-only drift gate for the duplicated `cargo-dylint` version pin.
///
/// Resolves every [`DYLINT_PIN_SITES`] file against the workspace root
/// (`manifest_dir/..`), extracts each `--version "..."` constraint and
/// returns `Err(Error::Parse)` naming both sides (the constant here and the
/// offending line) when any differs from `DYLINT_VERSION_CONSTRAINT`.
///
/// A pin site that does not exist is skipped, not failed: that is exactly
/// the packaged-crate branch of [`run()`] (`build.rs` selects `lint-data/`
/// when `../soroban_cost_lints/src/lib.rs` is absent), where `cargo package`
/// has stripped the workspace siblings from the shipped tarball. For the
/// same reason no `rerun-if-changed` directive is emitted for a missing
/// file — cargo treats a nonexistent watched path as always-dirty.
fn check_dylint_version_pins(manifest_dir: &Path) -> Result<()> {
    let workspace_root = manifest_dir.join("..");
    for rel in DYLINT_PIN_SITES {
        let path = workspace_root.join(rel);
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(Error::Parse(format!(
                    "Failed to read {}: {}",
                    path.display(),
                    e
                )));
            }
        };
        println!("cargo:rerun-if-changed=../{}", rel);
        for (lineno, constraint) in extract_version_constraints(&content) {
            if constraint != DYLINT_VERSION_CONSTRAINT {
                return Err(Error::Parse(format!(
                    "cargo-dylint version pin drift: `cargo-cost-lint/build.rs` advertises \
                     DYLINT_VERSION_CONSTRAINT \"{DYLINT_VERSION_CONSTRAINT}\" to `cargo \
                     cost-lint --version`, but the install line at {}:{} pins --version \
                     \"{}\". Update both sides together.",
                    path.display(),
                    lineno,
                    constraint
                )));
            }
        }
    }
    Ok(())
}

fn run() -> Result<()> {
    // cargo-cost-lint embeds compile-time metadata, documentation explanations,
    // and the pinned toolchain version. Inside the workspace these are read from
    // their sources of truth (`../soroban_cost_lints/src/lib.rs`, `../docs/lints`
    // and `../rust-toolchain`). `cargo package` only ships files inside this
    // package, so a packaged crate reads the snapshot in `lint-data/` instead.
    // `tests/lint_data_snapshot.rs` fails when the snapshot drifts from the
    // workspace sources; `make sync-lint-data` refreshes it.
    let manifest_dir_str = env::var("CARGO_MANIFEST_DIR").map_err(|_| Error::MissingEnv)?;
    let manifest_dir = PathBuf::from(manifest_dir_str);

    let workspace_lib_rs = manifest_dir.join("../soroban_cost_lints/src/lib.rs");
    let (lib_rs_path, docs_dir, toolchain_path, docs_rel) = if workspace_lib_rs.exists() {
        println!("cargo:rerun-if-changed=../soroban_cost_lints/src/lib.rs");
        println!("cargo:rerun-if-changed=../docs/lints");
        println!("cargo:rerun-if-changed=../rust-toolchain");
        (
            workspace_lib_rs,
            manifest_dir.join("../docs/lints"),
            manifest_dir.join("../rust-toolchain"),
            "../docs/lints",
        )
    } else {
        println!("cargo:rerun-if-changed=lint-data");
        (
            manifest_dir.join("lint-data/lib.rs"),
            manifest_dir.join("lint-data/docs"),
            manifest_dir.join("lint-data/rust-toolchain"),
            "lint-data/docs",
        )
    };

    // Gate the duplicated cargo-dylint version pin against the workspace CI
    // install lines. A no-op on the packaged-crate branch above, where the
    // pin sites are absent.
    check_dylint_version_pins(&manifest_dir)?;

    if !lib_rs_path.exists() {
        return Err(Error::Parse(format!(
            "cargo-cost-lint needs lint metadata from either the soroban-cost-linter \
             workspace (../soroban_cost_lints/src/lib.rs) or the bundled snapshot \
             ({}), but neither was found",
            lib_rs_path.display()
        )));
    }

    let content = fs::read_to_string(&lib_rs_path).map_err(|e| {
        Error::Parse(format!(
            "Failed to read source file {}: {}",
            lib_rs_path.display(),
            e
        ))
    })?;

    // --- Parse `lib.rs` once, then make the three views agree ---
    // `parse_register_lints` reads the registration lists, `parse_declare_lints`
    // the `declare_lint!` blocks and `parse_lint_metadata` the `LINT_METADATA`
    // registry. They are independent textual parsers, so every lint must show
    // up in all three or the build fails here rather than shipping an inventory
    // with a silent gap in it. Where two views carry the same field — a lint's
    // name, and its description — they must carry the same value.
    let names = parse_register_lints(&content)?;
    let declared = parse_declare_lints(&content)?;
    let registry = parse_lint_metadata(&content)?;

    let registered: HashSet<&str> = names.iter().map(String::as_str).collect();

    let missing_declare: Vec<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|name| !declared.iter().any(|meta| meta.name == *name))
        .collect();
    if !missing_declare.is_empty() {
        return Err(Error::Parse(format!(
            "lint(s) {} registered in lib.rs but missing a declare_lint! block",
            join_quoted(&missing_declare)
        )));
    }

    let undeclared: Vec<&str> = declared
        .iter()
        .map(|meta| meta.name.as_str())
        .filter(|name| !registered.contains(name))
        .collect();
    if !undeclared.is_empty() {
        return Err(Error::Parse(format!(
            "lint(s) {} have a declare_lint! block in lib.rs but are not registered",
            join_quoted(&undeclared)
        )));
    }

    let mut missing_category: Vec<&str> = registered
        .iter()
        .copied()
        .filter(|name| !registry.contains_key(*name))
        .collect();
    if !missing_category.is_empty() {
        missing_category.sort_unstable();
        return Err(Error::Parse(format!(
            "lint(s) {} are registered in lib.rs but have no LINT_METADATA row, so the \
             inventory would carry no category",
            join_quoted(&missing_category)
        )));
    }

    let mut orphan_category: Vec<&str> = registry
        .keys()
        .map(String::as_str)
        .filter(|name| !registered.contains(name))
        .collect();
    if !orphan_category.is_empty() {
        orphan_category.sort_unstable();
        return Err(Error::Parse(format!(
            "LINT_METADATA row(s) {} name a lint that is not registered in lib.rs",
            join_quoted(&orphan_category)
        )));
    }

    // Both views spell out a description for every lint, so a hand edit that
    // updates one and not the other would ship two contradictory descriptions
    // of the same lint. `declare_lint!` wins: it is what `--list-lints` and the
    // generated docs show.
    let mut divergent: Vec<String> = declared
        .iter()
        .filter_map(|meta| {
            let row = registry.get(meta.name.as_str())?;
            (row.description != meta.description).then(|| {
                format!(
                    "{}: declare_lint! has {:?}, LINT_METADATA has {:?}",
                    meta.name, meta.description, row.description
                )
            })
        })
        .collect();
    if !divergent.is_empty() {
        divergent.sort_unstable();
        let shown = divergent.len().min(5);
        let preview = divergent[..shown].join("; ");
        let rest = if divergent.len() > shown {
            format!(" (and {} more)", divergent.len() - shown)
        } else {
            String::new()
        };
        return Err(Error::Parse(format!(
            "lib.rs's `declare_lint!` blocks and `LINT_METADATA` rows disagree about a lint's \
             description: {}{}. Make the two descriptions identical",
            preview, rest
        )));
    }

    // Build a name→metadata lookup from the declare_lint! blocks.
    let metadata_by_name: HashMap<&str, &LintMeta> =
        declared.iter().map(|m| (m.name.as_str(), m)).collect();

    // Derive LINT_INFO in the same order as register_lints, so the three
    // lists can never drift. The presence checks above mean every lookup here
    // succeeds.
    let ordered: Vec<&LintMeta> = names
        .iter()
        .map(|name| {
            metadata_by_name
                .get(name.as_str())
                .copied()
                .expect("registered lints were checked against declare_lint! above")
        })
        .collect();

    // --- Verify every registered lint has a corresponding doc file ---
    for name in &names {
        let doc_path = docs_dir.join(format!("{}.md", name));
        assert!(
            doc_path.exists(),
            "lint '{}' is registered but has no doc file at '{}'. \
             Create a documentation page at docs/lints/{}.md to explain \
             what the lint does, why it is expensive, and how to fix it.",
            name,
            doc_path.display(),
            name
        );
    }

    // --- Verify no orphaned docs/lints/*.md exist without a registered lint ---
    if let Ok(read_dir) = fs::read_dir(&docs_dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("md")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && stem != "README"
                && !names.contains(&stem.to_lowercase())
            {
                eprintln!(
                    "warning: doc file '{:?}' exists in docs/lints/ but lint '{}' is not registered — skipping orphan check",
                    path, stem
                );
            }
        }
    }

    // --- Read each doc file and embed as raw string literals ---
    let mut explanations: Vec<(String, String)> = Vec::new();
    for name in &names {
        let doc_path = docs_dir.join(format!("{}.md", name));
        let doc_content = fs::read_to_string(&doc_path).unwrap_or_else(|e| {
            panic!(
                "Failed to read doc file '{}': expected it to be readable, got {}",
                doc_path.display(),
                e
            )
        });
        // Notify cargo to re-run build.rs when any doc file changes
        println!("cargo:rerun-if-changed={}/{}.md", docs_rel, name);
        explanations.push((name.clone(), doc_content));
    }

    // --- Parse the pinned toolchain and emit version metadata ---
    let toolchain_channel = parse_toolchain_channel(&toolchain_path)?;

    let out_dir = env::var_os("OUT_DIR").ok_or(Error::MissingEnv)?;
    let names_path = Path::new(&out_dir).join("lint_names.rs");
    let metadata_path = Path::new(&out_dir).join("lint_metadata.rs");
    let info_path = Path::new(&out_dir).join("lint_info.rs");
    let explanations_path = Path::new(&out_dir).join("lint_explanations.rs");
    let version_path = Path::new(&out_dir).join("version_info.rs");

    // Emit version_info.rs with toolchain and dylint constraint.
    let version_out = format!(
        "pub const PINNED_TOOLCHAIN: &str = \"{}\";\n\npub const DYLINT_VERSION_CONSTRAINT: &str = \"{}\";\n",
        toolchain_channel, DYLINT_VERSION_CONSTRAINT
    );
    fs::write(&version_path, version_out)
        .map_err(|e| Error::Parse(format!("Failed to write version_info.rs: {}", e)))?;

    // Emit LINT_NAMES (used by the filter logic in main.rs).
    let mut names_out = String::new();
    names_out.push_str("pub const LINT_NAMES: &[&str] = &[\n");
    for name in &names {
        names_out.push_str(&format!("    \"{}\",\n", name));
    }
    names_out.push_str("];\n");
    fs::write(&names_path, names_out)
        .map_err(|e| Error::Parse(format!("Failed to write lint_names.rs: {}", e)))?;

    let mut metadata_out = String::new();
    metadata_out.push_str("#[derive(Serialize, Debug)]\npub struct LintInventoryEntry {\n");
    metadata_out.push_str("    pub name: &'static str,\n");
    metadata_out.push_str("    pub default_level: &'static str,\n");
    metadata_out.push_str("    pub description: &'static str,\n");
    metadata_out.push_str("    pub category: &'static str,\n");
    metadata_out.push_str("    pub documentation_url: &'static str,\n");
    metadata_out.push_str("}\n\n");
    metadata_out.push_str("#[derive(Serialize, Debug)]\npub struct LintInventory {\n");
    metadata_out.push_str("    pub version: &'static str,\n");
    metadata_out.push_str("    pub schema: &'static str,\n");
    metadata_out.push_str("    pub lints: &'static [LintInventoryEntry],\n");
    metadata_out.push_str("}\n\n");
    metadata_out.push_str("pub const LINT_INVENTORY: LintInventory = LintInventory {\n");
    metadata_out.push_str("    version: \"1.0\",\n");
    metadata_out.push_str("    schema: \"https://github.com/Tollcraft/soroban-cost-linter/blob/main/docs/lints/README.md#lint-inventory-schema\",\n");
    metadata_out.push_str("    lints: &[\n");

    for name in &names {
        // Both lookups were validated against `names` above, so this loop can
        // only fail if the cross-checks were skipped.
        let meta = metadata_by_name.get(name.as_str()).ok_or_else(|| {
            Error::Parse(format!(
                "lint '{}' registered in lib.rs but metadata not found in declare_lint! blocks",
                name
            ))
        })?;
        let category = registry
            .get(name)
            .map(|row| row.category.as_str())
            .ok_or_else(|| {
                Error::Parse(format!(
                    "lint '{}' registered in lib.rs but has no LINT_METADATA row",
                    name
                ))
            })?;
        let docs_path = format!(
            "https://github.com/Tollcraft/soroban-cost-linter/blob/main/docs/lints/{}.md",
            name
        );
        metadata_out.push_str("        LintInventoryEntry {\n");
        metadata_out.push_str(&format!("            name: {},\n", rust_string(name)));
        metadata_out.push_str(&format!(
            "            default_level: {},\n",
            rust_string(&meta.level)
        ));
        metadata_out.push_str(&format!(
            "            description: {},\n",
            rust_string(&meta.description)
        ));
        metadata_out.push_str(&format!(
            "            category: {},\n",
            rust_string(category)
        ));
        metadata_out.push_str(&format!(
            "            documentation_url: {},\n",
            rust_string(&docs_path)
        ));
        metadata_out.push_str("        },\n");
    }
    metadata_out.push_str("    ],\n");
    metadata_out.push_str("};\n");
    fs::write(&metadata_path, metadata_out)
        .map_err(|e| Error::Parse(format!("Failed to write lint_metadata.rs: {}", e)))?;

    // Emit LintInfo/LINT_INFO for --list-lints (included by main.rs).
    let mut info_out = String::new();
    info_out.push_str("pub struct LintInfo {\n");
    info_out.push_str("    pub name: &'static str,\n");
    info_out.push_str("    pub level: &'static str,\n");
    info_out.push_str("    pub description: &'static str,\n");
    info_out.push_str("}\n\n");
    info_out.push_str("pub const LINT_INFO: &[LintInfo] = &[\n");
    for lint in &ordered {
        info_out.push_str("    LintInfo {\n");
        info_out.push_str(&format!("        name: \"{}\",\n", lint.name));
        info_out.push_str(&format!("        level: \"{}\",\n", lint.level));
        info_out.push_str(&format!("        description: \"{}\",\n", lint.description));
        info_out.push_str("    },\n");
    }
    info_out.push_str("];\n");
    fs::write(&info_path, info_out)
        .map_err(|e| Error::Parse(format!("Failed to write lint_info.rs: {}", e)))?;

    // --- Write lint_explanations.rs with embedded doc content as raw string literals ---
    let mut explanations_out = String::new();
    explanations_out.push_str("#[derive(Serialize, Debug)]\npub struct LintExplanation {\n");
    explanations_out.push_str("    pub name: &'static str,\n");
    explanations_out.push_str("    pub markdown: &'static str,\n");
    explanations_out.push_str("}\n\n");
    explanations_out.push_str("pub const LINT_EXPLANATIONS: &[LintExplanation] = &[\n");
    for (name, doc_content) in &explanations {
        let escaped = raw_string_literal(doc_content);
        explanations_out.push_str("    LintExplanation {\n");
        explanations_out.push_str(&format!("        name: \"{}\",\n", name));
        explanations_out.push_str(&format!("        markdown: {},\n", escaped));
        explanations_out.push_str("    },\n");
    }
    explanations_out.push_str("];\n");
    fs::write(&explanations_path, explanations_out)
        .map_err(|e| Error::Parse(format!("Failed to write lint_explanations.rs: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // `run()` reads process-wide environment variables, so the tests that set
    // them must not overlap.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn declare(name: &str, level: &str, description: &str) -> String {
        format!("declare_lint! {{\n    pub {name},\n    {level},\n    \"{description}\"\n}}\n")
    }

    fn row(name: &str, category: &str, description: &str) -> String {
        format!(
            "    LintMeta {{\n        name: \"{name}\",\n        category: LintCategory::{category},\n        description: \"{description}\",\n    }},\n"
        )
    }

    fn dylint_list(names: &[&str]) -> String {
        format!(
            "dylint_lint_impl! {{\n    soroban_cost_lints,\n    [\n{}    ]\n}}\n",
            names
                .iter()
                .map(|n| format!("        {n},\n"))
                .collect::<String>()
        )
    }

    /// A minimal, self-consistent `lib.rs` with two lints.
    fn valid_lib() -> String {
        format!(
            "{}{}{}pub const LINT_METADATA: &[LintMeta] = &[\n{}{}];\n",
            declare("LINT_A", "Warn", "does a"),
            declare("LINT_B", "Deny", "does b"),
            dylint_list(&["LINT_A", "LINT_B"]),
            row("lint_a", "Storage", "does a"),
            row("lint_b", "Compute", "does b"),
        )
    }

    // ---- small helpers -------------------------------------------------

    #[test]
    fn rust_string_escapes_quotes_and_newlines() {
        assert_eq!(rust_string("plain"), "\"plain\"");
        assert_eq!(rust_string("a\"b\nc\\"), "\"a\\\"b\\nc\\\\\"");
        assert_eq!(rust_string(""), "\"\"");
    }

    #[test]
    fn join_quoted_formats_names() {
        assert_eq!(join_quoted(&[]), "");
        assert_eq!(join_quoted(&["a"]), "\"a\"");
        assert_eq!(join_quoted(&["a", "b"]), "\"a\", \"b\"");
    }

    #[test]
    fn sorted_diff_is_sorted_and_directional() {
        let a: HashSet<&str> = ["c", "a", "b"].into_iter().collect();
        let b: HashSet<&str> = ["b"].into_iter().collect();
        assert_eq!(sorted_diff(&a, &b), vec!["a", "c"]);
        assert!(sorted_diff(&b, &a).is_empty());
    }

    #[test]
    fn raw_string_literal_picks_shortest_delimiter() {
        assert_eq!(raw_string_literal("abc"), "r\"abc\"");
        assert_eq!(raw_string_literal("a\"b"), "r#\"a\"b\"#");
        assert_eq!(raw_string_literal("a\"#b"), "r##\"a\"#b\"##");
        assert_eq!(raw_string_literal(""), "r\"\"");
    }

    #[test]
    fn error_display_and_source() {
        let io = Error::from(std::io::Error::other("boom"));
        assert_eq!(io.to_string(), "I/O error: boom");
        assert!(std::error::Error::source(&io).is_some());

        let env = Error::MissingEnv;
        assert!(env.to_string().contains("OUT_DIR"));
        assert!(std::error::Error::source(&env).is_none());

        let parse = Error::Parse("bad".into());
        assert_eq!(parse.to_string(), "Parse error: bad");
        assert!(std::error::Error::source(&parse).is_none());
    }

    // ---- registration parsers -----------------------------------------

    #[test]
    fn dylint_impl_absent_is_none() {
        assert!(parse_dylint_impl("fn main() {}").unwrap().is_none());
    }

    #[test]
    fn dylint_impl_lowercases_and_skips_comments_and_blanks() {
        let src = "dylint_lint_impl! {\n    krate,\n    [\n        // note\n\n        LINT_A,\n        Lint_B\n    ]\n}";
        assert_eq!(
            parse_dylint_impl(src).unwrap().unwrap(),
            vec!["lint_a", "lint_b"]
        );
    }

    #[test]
    fn dylint_impl_errors_without_list_or_close() {
        assert!(matches!(
            parse_dylint_impl("dylint_lint_impl! { krate }"),
            Err(Error::Parse(m)) if m.contains("no lint list")
        ));
        assert!(matches!(
            parse_dylint_impl("dylint_lint_impl! { krate, [ A, B "),
            Err(Error::Parse(m)) if m.contains("not closed")
        ));
    }

    #[test]
    fn legacy_register_absent_is_none() {
        assert!(parse_legacy_register_lints("nothing").unwrap().is_none());
    }

    #[test]
    fn legacy_register_parses_list() {
        let src = "lint_store.register_lints(&[\n    A,\n    // skip\n    B,\n]);";
        assert_eq!(
            parse_legacy_register_lints(src).unwrap().unwrap(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn legacy_register_errors_when_unterminated() {
        assert!(matches!(
            parse_legacy_register_lints("lint_store.register_lints(&[ A,"),
            Err(Error::Parse(m)) if m.contains("end of register_lints")
        ));
    }

    #[test]
    fn register_lints_requires_some_source() {
        assert!(matches!(
            parse_register_lints("fn main() {}"),
            Err(Error::Parse(m)) if m.contains("Could not find")
        ));
    }

    #[test]
    fn register_lints_uses_whichever_list_exists() {
        let dylint = dylint_list(&["A", "B"]);
        assert_eq!(parse_register_lints(&dylint).unwrap(), vec!["a", "b"]);
        let legacy = "lint_store.register_lints(&[\n    B,\n    A,\n]);";
        assert_eq!(parse_register_lints(legacy).unwrap(), vec!["b", "a"]);
    }

    #[test]
    fn register_lints_accepts_identical_lists() {
        let src = format!(
            "{}lint_store.register_lints(&[\n    A,\n    B,\n]);",
            dylint_list(&["A", "B"])
        );
        assert_eq!(parse_register_lints(&src).unwrap(), vec!["a", "b"]);
    }

    #[test]
    fn register_lints_rejects_different_sets() {
        let src = format!(
            "{}lint_store.register_lints(&[\n    A,\n    C,\n]);",
            dylint_list(&["A", "B"])
        );
        match parse_register_lints(&src) {
            Err(Error::Parse(m)) => {
                assert!(m.contains("disagree"), "{m}");
                assert!(m.contains("Only in `dylint_lint_impl!`: [b]"), "{m}");
                assert!(m.contains("Only in `register_lints`: [c]"), "{m}");
            }
            other => panic!("expected disagreement error, got {other:?}"),
        }
    }

    #[test]
    fn register_lints_rejects_same_set_in_different_order() {
        let src = format!(
            "{}lint_store.register_lints(&[\n    B,\n    A,\n]);",
            dylint_list(&["A", "B"])
        );
        assert!(matches!(
            parse_register_lints(&src),
            Err(Error::Parse(m)) if m.contains("different orders")
        ));
    }

    // ---- declare_lint! parser -----------------------------------------

    #[test]
    fn declare_lints_parses_blocks_in_order() {
        let metas = parse_declare_lints(&valid_lib()).unwrap();
        assert_eq!(metas.len(), 2);
        assert_eq!(
            (metas[0].name.as_str(), metas[0].level.as_str()),
            ("lint_a", "warn")
        );
        assert_eq!(metas[0].description, "does a");
        assert_eq!(metas[1].name, "lint_b");
        assert_eq!(metas[1].level, "deny");
    }

    #[test]
    fn declare_lints_without_blocks_is_empty() {
        assert!(parse_declare_lints("fn x() {}").unwrap().is_empty());
    }

    #[test]
    fn declare_lints_ignores_comments_attributes_and_braces_in_strings() {
        let src = "declare_lint! {\n    /// doc\n    #[allow(x)]\n    // c\n    /* b */\n    pub LINT_A,\n    Warn,\n    \"has } brace\"\n}\n";
        let metas = parse_declare_lints(src).unwrap();
        assert_eq!(metas[0].name, "lint_a");
        assert_eq!(metas[0].description, "has } brace");
    }

    #[test]
    fn declare_lints_joins_multiline_descriptions() {
        let src = "declare_lint! {\n    pub LINT_A,\n    Warn,\n    \"first\"\n    \"second\"\n}\n";
        let metas = parse_declare_lints(src).unwrap();
        // Lines are joined with a space; only the outermost quotes are trimmed.
        assert_eq!(metas[0].description, "first\" \"second");
    }

    #[test]
    fn declare_lints_accepts_unquoted_description() {
        let src = "declare_lint! {\n    pub LINT_A,\n    Warn,\n    plain text,\n}\n";
        assert_eq!(
            parse_declare_lints(src).unwrap()[0].description,
            "plain text"
        );
    }

    #[test]
    fn declare_lints_reports_malformed_blocks() {
        let unclosed = "declare_lint! {\n    pub A,\n    Warn,\n    \"d\"\n";
        assert!(matches!(
            parse_declare_lints(unclosed),
            Err(Error::Parse(m)) if m.contains("unclosed declare_lint! block starting at line 1")
        ));

        let short = "declare_lint! {\n    pub A,\n    Warn,\n}\n";
        assert!(matches!(
            parse_declare_lints(short),
            Err(Error::Parse(m)) if m.contains("fewer than 3")
        ));

        let empty_name = "declare_lint! {\n    pub ,\n    Warn,\n    \"d\"\n}\n";
        assert!(matches!(
            parse_declare_lints(empty_name),
            Err(Error::Parse(m)) if m.contains("empty lint name")
        ));

        let empty_level = "declare_lint! {\n    pub A,\n    ,\n    \"d\"\n}\n";
        assert!(matches!(
            parse_declare_lints(empty_level),
            Err(Error::Parse(m)) if m.contains("empty lint level")
        ));

        let empty_desc = "declare_lint! {\n    pub A,\n    Warn,\n    \"\"\n}\n";
        assert!(matches!(
            parse_declare_lints(empty_desc),
            Err(Error::Parse(m)) if m.contains("empty description")
        ));
    }

    #[test]
    fn declare_lints_reports_line_of_second_block() {
        let src = format!(
            "{}\ndeclare_lint! {{\n    pub B,\n    Warn,\n}}\n",
            declare("A", "Warn", "d")
        );
        assert!(matches!(
            parse_declare_lints(&src),
            Err(Error::Parse(m)) if m.contains("line 7")
        ));
    }

    // ---- LINT_METADATA parser -----------------------------------------

    #[test]
    fn lint_metadata_parses_rows() {
        let rows = parse_lint_metadata(&valid_lib()).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows["lint_a"].category, "Storage");
        assert_eq!(rows["lint_b"].description, "does b");
    }

    #[test]
    fn lint_metadata_requires_registry_and_slice_literal() {
        assert!(matches!(
            parse_lint_metadata("fn x() {}"),
            Err(Error::Parse(m)) if m.contains("no `pub const LINT_METADATA`")
        ));
        assert!(matches!(
            parse_lint_metadata("pub const LINT_METADATA: X = other();"),
            Err(Error::Parse(m)) if m.contains("not a slice literal")
        ));
        assert!(matches!(
            parse_lint_metadata("pub const LINT_METADATA: X = &[ LintMeta {"),
            Err(Error::Parse(_))
        ));
    }

    #[test]
    fn lint_metadata_rejects_empty_registry() {
        assert!(matches!(
            parse_lint_metadata("pub const LINT_METADATA: X = &[];"),
            Err(Error::Parse(m)) if m.contains("no entries")
        ));
    }

    #[test]
    fn lint_metadata_rejects_incomplete_and_empty_rows() {
        let missing = "pub const LINT_METADATA: X = &[\n    LintMeta {\n        name: \"a\",\n        category: LintCategory::Storage,\n    },\n];";
        assert!(matches!(
            parse_lint_metadata(missing),
            Err(Error::Parse(m)) if m.contains("Could not parse a LINT_METADATA entry")
        ));

        let empty = format!(
            "pub const LINT_METADATA: X = &[\n{}];",
            row("a", "Storage", "")
        );
        assert!(matches!(
            parse_lint_metadata(&empty),
            Err(Error::Parse(m)) if m.contains("empty name, category or description")
        ));
    }

    #[test]
    fn lint_metadata_rejects_duplicates_case_insensitively() {
        let src = format!(
            "pub const LINT_METADATA: X = &[\n{}{}];",
            row("dup", "Storage", "one"),
            row("DUP", "Compute", "two")
        );
        assert!(matches!(
            parse_lint_metadata(&src),
            Err(Error::Parse(m)) if m.contains("duplicate LINT_METADATA row")
        ));
    }

    #[test]
    fn lint_metadata_accepts_bare_category() {
        let src = "pub const LINT_METADATA: X = &[\n    LintMeta {\n        name: \"a\",\n        category: Storage,\n        description: \"d\",\n    },\n];";
        assert_eq!(parse_lint_metadata(src).unwrap()["a"].category, "Storage");
    }

    // ---- matching_delimiter and literal scanners ------------------------

    #[test]
    fn matching_delimiter_handles_each_bracket_kind_and_nesting() {
        assert_eq!(matching_delimiter("(a(b)c)", 0), Ok(6));
        assert_eq!(matching_delimiter("[a[b]c]", 0), Ok(6));
        assert_eq!(matching_delimiter("{a{b}c}", 0), Ok(6));
        assert_eq!(matching_delimiter("x{}", 1), Ok(2));
    }

    #[test]
    fn matching_delimiter_rejects_bad_starts() {
        assert!(
            matching_delimiter("abc", 0)
                .unwrap_err()
                .contains("expected")
        );
        assert!(
            matching_delimiter("{", 5)
                .unwrap_err()
                .contains("character boundary")
        );
        assert!(
            matching_delimiter("é{", 1)
                .unwrap_err()
                .contains("character boundary")
        );
        assert!(
            matching_delimiter("", 0)
                .unwrap_err()
                .contains("no delimiter")
        );
        assert!(
            matching_delimiter("{", 1)
                .unwrap_err()
                .contains("no delimiter")
        );
    }

    #[test]
    fn matching_delimiter_reports_unclosed() {
        assert!(
            matching_delimiter("{ {}", 0)
                .unwrap_err()
                .contains("closes")
        );
    }

    #[test]
    fn matching_delimiter_skips_comments() {
        assert_eq!(matching_delimiter("{ // }\n }", 0), Ok(8));
        assert_eq!(matching_delimiter("{ /* } /* } */ } */ }", 0), Ok(20));
        assert!(matching_delimiter("{ // }", 0).is_err());
        assert!(
            matching_delimiter("{ /* never closed", 0)
                .unwrap_err()
                .contains("unterminated block comment")
        );
    }

    #[test]
    fn matching_delimiter_skips_string_char_and_raw_literals() {
        assert_eq!(matching_delimiter("{ \"}\" }", 0), Ok(6));
        assert_eq!(matching_delimiter("{ \"\\\"}\" }", 0), Ok(8));
        assert_eq!(matching_delimiter("{ '}' }", 0), Ok(6));
        assert_eq!(matching_delimiter("{ '\\n' }", 0), Ok(7));
        assert_eq!(matching_delimiter("{ r\"}\" }", 0), Ok(7));
        assert_eq!(matching_delimiter("{ r#\"}\"# }", 0), Ok(9));
        // A lifetime is not a char literal, and `r` alone is just an identifier.
        assert_eq!(matching_delimiter("{ &'a x }", 0), Ok(8));
        assert_eq!(matching_delimiter("{ r }", 0), Ok(4));
        assert!(
            matching_delimiter("{ \"open", 0)
                .unwrap_err()
                .contains("unterminated string")
        );
    }

    #[test]
    fn matching_delimiter_handles_multibyte_text() {
        assert_eq!(matching_delimiter("{ é }", 0), Ok(5));
    }

    #[test]
    fn skip_quoted_honours_escapes() {
        assert_eq!(skip_quoted("\"ab\" x", 0), Ok(4));
        assert_eq!(skip_quoted("\"a\\\"b\"", 0), Ok(6));
        assert!(skip_quoted("\"abc", 0).is_err());
        assert!(skip_quoted("\"abc\\", 0).is_err());
    }

    #[test]
    fn char_literal_end_distinguishes_lifetimes() {
        assert_eq!(char_literal_end("'a'", 0), Some(3));
        assert_eq!(char_literal_end("'é'", 0), Some(4));
        assert_eq!(char_literal_end("'\\n'", 0), Some(4));
        assert_eq!(char_literal_end("'\\u{1F600}'", 0), Some(11));
        assert_eq!(char_literal_end("'a ", 0), None);
        assert_eq!(char_literal_end("'", 0), None);
        assert_eq!(char_literal_end("'\\", 0), None);
        assert_eq!(char_literal_end("'\\n", 0), None);
        assert_eq!(char_literal_end("'\\u{1\n}'", 0), None);
    }

    #[test]
    fn raw_string_end_finds_terminator() {
        assert_eq!(raw_string_end("r\"ab\"", 0), Some(5));
        assert_eq!(raw_string_end("r#\"a\"b\"#", 0), Some(8));
        assert_eq!(raw_string_end("r##\"a\"#b\"##", 0), Some(11));
        assert_eq!(raw_string_end("r\"open", 0), None);
        assert_eq!(raw_string_end("r#\"open\"", 0), None);
        assert_eq!(raw_string_end("rx", 0), None);
        assert_eq!(raw_string_end("r#x", 0), None);
        assert_eq!(raw_string_end("r", 0), None);
    }

    // ---- toolchain parser ---------------------------------------------

    fn write(dir: &Path, rel: &str, body: &str) -> PathBuf {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn toolchain_channel_is_extracted() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "rust-toolchain",
            "[toolchain]\nchannel = \"nightly-2026-04-16\"\ncomponents = [\"rustfmt\"]\n",
        );
        assert_eq!(
            parse_toolchain_channel(&path).unwrap(),
            "nightly-2026-04-16"
        );
    }

    #[test]
    fn toolchain_channel_tolerates_spacing() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "t", "   channel   =   \"stable\"  \n");
        assert_eq!(parse_toolchain_channel(&path).unwrap(), "stable");
    }

    #[test]
    fn toolchain_channel_errors() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("absent");
        assert!(matches!(
            parse_toolchain_channel(&missing),
            Err(Error::Parse(m)) if m.contains("Failed to read")
        ));
        for body in [
            "[toolchain]\n",
            "channel nightly\n",
            "channel = nightly\n",
            "channel = \"unterminated\n",
        ] {
            let path = write(dir.path(), "bad", body);
            assert!(
                matches!(parse_toolchain_channel(&path), Err(Error::Parse(m)) if m.contains("Could not find 'channel'")),
                "body {body:?} should be rejected"
            );
        }
    }

    // ---- run() end to end ---------------------------------------------

    struct Fixture {
        _dir: tempfile::TempDir,
        pkg: PathBuf,
        out: PathBuf,
    }

    /// Lays out a packaged-crate tree (`lint-data/` snapshot, no workspace
    /// sibling) so `run()` takes the snapshot branch.
    fn fixture(lib: &str, docs: &[&str]) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join("pkg");
        let out = dir.path().join("out");
        fs::create_dir_all(&out).unwrap();
        write(&pkg, "lint-data/lib.rs", lib);
        write(
            &pkg,
            "lint-data/rust-toolchain",
            "[toolchain]\nchannel = \"nightly-test\"\n",
        );
        for name in docs {
            write(
                &pkg,
                &format!("lint-data/docs/{name}.md"),
                &format!("# {name}\n"),
            );
        }
        Fixture {
            _dir: dir,
            pkg,
            out,
        }
    }

    fn run_with(manifest: Option<&Path>, out: Option<&Path>) -> Result<()> {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: ENV_LOCK serialises every test that touches these variables.
        unsafe {
            match manifest {
                Some(p) => env::set_var("CARGO_MANIFEST_DIR", p),
                None => env::remove_var("CARGO_MANIFEST_DIR"),
            }
            match out {
                Some(p) => env::set_var("OUT_DIR", p),
                None => env::remove_var("OUT_DIR"),
            }
        }
        run()
    }

    fn parse_err(result: Result<()>) -> String {
        match result {
            Err(Error::Parse(m)) => m,
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn run_generates_all_outputs() {
        let f = fixture(&valid_lib(), &["lint_a", "lint_b", "README"]);
        run_with(Some(&f.pkg), Some(&f.out)).unwrap();

        let read = |n: &str| fs::read_to_string(f.out.join(n)).unwrap();
        let names = read("lint_names.rs");
        assert!(names.contains("\"lint_a\",") && names.contains("\"lint_b\","));
        let info = read("lint_info.rs");
        assert!(info.contains("level: \"warn\"") && info.contains("level: \"deny\""));
        assert!(read("lint_metadata.rs").contains("category: \"Storage\""));
        assert!(read("lint_explanations.rs").contains("r\"# lint_a\n\""));
        let version = read("version_info.rs");
        assert!(version.contains("PINNED_TOOLCHAIN: &str = \"nightly-test\""));
        assert!(version.contains(DYLINT_VERSION_CONSTRAINT));
    }

    #[test]
    fn run_tolerates_orphan_docs() {
        let f = fixture(&valid_lib(), &["lint_a", "lint_b", "stray"]);
        write(&f.pkg, "lint-data/docs/notes.txt", "ignored");
        run_with(Some(&f.pkg), Some(&f.out)).unwrap();
    }

    #[test]
    fn run_requires_environment() {
        let f = fixture(&valid_lib(), &["lint_a", "lint_b"]);
        assert!(matches!(
            run_with(None, Some(&f.out)),
            Err(Error::MissingEnv)
        ));
        assert!(matches!(
            run_with(Some(&f.pkg), None),
            Err(Error::MissingEnv)
        ));
    }

    #[test]
    fn run_requires_a_lint_source() {
        let f = fixture(&valid_lib(), &[]);
        fs::remove_file(f.pkg.join("lint-data/lib.rs")).unwrap();
        assert!(parse_err(run_with(Some(&f.pkg), Some(&f.out))).contains("neither was found"));
    }

    #[test]
    fn run_reports_unreadable_source() {
        let f = fixture(&valid_lib(), &[]);
        fs::remove_file(f.pkg.join("lint-data/lib.rs")).unwrap();
        fs::create_dir(f.pkg.join("lint-data/lib.rs")).unwrap();
        assert!(
            parse_err(run_with(Some(&f.pkg), Some(&f.out))).contains("Failed to read source file")
        );
    }

    #[test]
    fn run_rejects_registered_lint_without_declare_block() {
        let lib = format!(
            "{}{}pub const LINT_METADATA: &[LintMeta] = &[\n{}{}];\n",
            declare("LINT_A", "Warn", "does a"),
            dylint_list(&["LINT_A", "LINT_B"]),
            row("lint_a", "Storage", "does a"),
            row("lint_b", "Storage", "does b"),
        );
        let f = fixture(&lib, &["lint_a", "lint_b"]);
        let msg = parse_err(run_with(Some(&f.pkg), Some(&f.out)));
        assert!(msg.contains("\"lint_b\"") && msg.contains("missing a declare_lint! block"));
    }

    #[test]
    fn run_rejects_declared_but_unregistered_lint() {
        let lib = format!(
            "{}{}{}pub const LINT_METADATA: &[LintMeta] = &[\n{}];\n",
            declare("LINT_A", "Warn", "does a"),
            declare("LINT_B", "Warn", "does b"),
            dylint_list(&["LINT_A"]),
            row("lint_a", "Storage", "does a"),
        );
        let f = fixture(&lib, &["lint_a"]);
        let msg = parse_err(run_with(Some(&f.pkg), Some(&f.out)));
        assert!(msg.contains("\"lint_b\"") && msg.contains("are not registered"));
    }

    #[test]
    fn run_rejects_registered_lint_without_metadata_row() {
        let lib = format!(
            "{}{}{}pub const LINT_METADATA: &[LintMeta] = &[\n{}];\n",
            declare("LINT_A", "Warn", "does a"),
            declare("LINT_B", "Warn", "does b"),
            dylint_list(&["LINT_A", "LINT_B"]),
            row("lint_a", "Storage", "does a"),
        );
        let f = fixture(&lib, &["lint_a", "lint_b"]);
        assert!(parse_err(run_with(Some(&f.pkg), Some(&f.out))).contains("no LINT_METADATA row"));
    }

    #[test]
    fn run_rejects_orphan_metadata_row() {
        let lib = format!(
            "{}{}pub const LINT_METADATA: &[LintMeta] = &[\n{}{}];\n",
            declare("LINT_A", "Warn", "does a"),
            dylint_list(&["LINT_A"]),
            row("lint_a", "Storage", "does a"),
            row("ghost", "Storage", "boo"),
        );
        let f = fixture(&lib, &["lint_a"]);
        let msg = parse_err(run_with(Some(&f.pkg), Some(&f.out)));
        assert!(msg.contains("\"ghost\"") && msg.contains("not registered"));
    }

    #[test]
    fn run_rejects_divergent_descriptions_and_truncates_preview() {
        let lib = format!(
            "{}{}pub const LINT_METADATA: &[LintMeta] = &[\n{}{}];\n",
            declare("LINT_A", "Warn", "does a"),
            dylint_list(&["LINT_A"]),
            row("lint_a", "Storage", "different"),
            "",
        );
        let f = fixture(&lib, &["lint_a"]);
        let msg = parse_err(run_with(Some(&f.pkg), Some(&f.out)));
        assert!(
            msg.contains("lint_a: declare_lint! has \"does a\", LINT_METADATA has \"different\"")
        );
        assert!(!msg.contains("more)"));

        // Seven divergent lints: only five are previewed.
        let names: Vec<String> = (0..7).map(|i| format!("l{i}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let upper: Vec<String> = names.iter().map(|n| n.to_uppercase()).collect();
        let upper_refs: Vec<&str> = upper.iter().map(String::as_str).collect();
        let mut lib = String::new();
        for n in &upper {
            lib.push_str(&declare(n, "Warn", "a"));
        }
        lib.push_str(&dylint_list(&upper_refs));
        lib.push_str("pub const LINT_METADATA: &[LintMeta] = &[\n");
        for n in &refs {
            lib.push_str(&row(n, "Storage", "b"));
        }
        lib.push_str("];\n");
        let f = fixture(&lib, &refs);
        assert!(parse_err(run_with(Some(&f.pkg), Some(&f.out))).contains("(and 2 more)"));
    }

    #[test]
    fn run_panics_on_missing_doc_file() {
        let f = fixture(&valid_lib(), &["lint_a"]);
        let result = std::panic::catch_unwind(|| run_with(Some(&f.pkg), Some(&f.out)));
        let payload = result.expect_err("missing doc must fail the build");
        let msg = payload
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_default();
        assert!(
            msg.contains("lint 'lint_b' is registered but has no doc file"),
            "{msg}"
        );
    }

    #[test]
    fn run_panics_on_unreadable_doc_file() {
        let f = fixture(&valid_lib(), &["lint_a", "lint_b"]);
        // A directory named `lint_b.md` exists but cannot be read as text.
        fs::remove_file(f.pkg.join("lint-data/docs/lint_b.md")).unwrap();
        fs::create_dir(f.pkg.join("lint-data/docs/lint_b.md")).unwrap();
        let result = std::panic::catch_unwind(|| run_with(Some(&f.pkg), Some(&f.out)));
        assert!(result.is_err());
    }

    #[test]
    fn run_reports_missing_toolchain_and_unwritable_output() {
        let f = fixture(&valid_lib(), &["lint_a", "lint_b"]);
        fs::remove_file(f.pkg.join("lint-data/rust-toolchain")).unwrap();
        assert!(parse_err(run_with(Some(&f.pkg), Some(&f.out))).contains("Failed to read"));

        let f = fixture(&valid_lib(), &["lint_a", "lint_b"]);
        let missing_out = f.out.join("does/not/exist");
        assert!(
            parse_err(run_with(Some(&f.pkg), Some(&missing_out)))
                .contains("Failed to write version_info.rs")
        );
    }

    #[test]
    fn run_prefers_workspace_sources_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join("cargo-cost-lint");
        let out = dir.path().join("out");
        fs::create_dir_all(&pkg).unwrap();
        fs::create_dir_all(&out).unwrap();
        write(dir.path(), "soroban_cost_lints/src/lib.rs", &valid_lib());
        write(
            dir.path(),
            "rust-toolchain",
            "[toolchain]\nchannel = \"ws-channel\"\n",
        );
        write(dir.path(), "docs/lints/lint_a.md", "a");
        write(dir.path(), "docs/lints/lint_b.md", "b");
        run_with(Some(&pkg), Some(&out)).unwrap();
        assert!(
            fs::read_to_string(out.join("version_info.rs"))
                .unwrap()
                .contains("ws-channel")
        );
    }

    // ---- cargo-dylint pin gate ------------------------------------------

    #[test]
    fn extract_version_constraints_finds_every_pin() {
        let content = "\
run: cargo install cargo-dylint dylint-link --version \"^6.0.1\" --locked
two on one line: --version \"a\" and --version \"b\"
unterminated --version \"oops
no flag here\n";
        assert_eq!(
            extract_version_constraints(content),
            vec![
                (1, "^6.0.1".to_string()),
                (2, "a".to_string()),
                (2, "b".to_string()),
            ]
        );
    }

    #[test]
    fn pin_gate_accepts_matching_site_and_skips_missing_sites() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let pkg = root.join("cargo-cost-lint");
        fs::create_dir_all(&pkg).unwrap();
        write(
            root,
            DYLINT_PIN_SITES[0],
            "        run: cargo install cargo-dylint dylint-link --version \"^6.0.1\" --locked\n",
        );
        assert!(check_dylint_version_pins(&pkg).is_ok());

        // No site exists at all: the packaged-crate branch, a no-op not a failure.
        let bare = tempfile::tempdir().unwrap();
        assert!(check_dylint_version_pins(&bare.path().join("pkg")).is_ok());
    }

    #[test]
    fn pin_gate_reports_drift_naming_both_sides() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let pkg = root.join("cargo-cost-lint");
        fs::create_dir_all(&pkg).unwrap();
        write(
            root,
            "action.yml",
            "header\n    run: cargo install cargo-dylint dylint-link --version \"^5.0.0\" --locked\n",
        );
        let msg = parse_err(check_dylint_version_pins(&pkg));
        assert!(msg.contains(DYLINT_VERSION_CONSTRAINT), "{msg}");
        assert!(msg.contains("\"^5.0.0\""), "{msg}");
        assert!(msg.contains("action.yml:2"), "{msg}");
    }

    #[test]
    fn pin_gate_surfaces_unreadable_site() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let pkg = root.join("cargo-cost-lint");
        fs::create_dir_all(&pkg).unwrap();
        fs::create_dir_all(root.join("action.yml")).unwrap();
        assert!(parse_err(check_dylint_version_pins(&pkg)).contains("Failed to read"));
    }

    #[test]
    fn run_passes_gate_on_packaged_crate_snapshot() {
        // `fixture()` lays out a packaged tree: every DYLINT_PIN_SITES file is
        // absent, so the gate is skipped, not failed, on the snapshot branch.
        let f = fixture(&valid_lib(), &["lint_a", "lint_b"]);
        run_with(Some(&f.pkg), Some(&f.out)).unwrap();
    }

    #[test]
    fn run_fails_when_a_workspace_pin_site_drifts_from_build_rs() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join("cargo-cost-lint");
        let out = dir.path().join("out");
        fs::create_dir_all(&pkg).unwrap();
        fs::create_dir_all(&out).unwrap();
        write(dir.path(), "soroban_cost_lints/src/lib.rs", &valid_lib());
        write(
            dir.path(),
            "rust-toolchain",
            "[toolchain]\nchannel = \"ws-channel\"\n",
        );
        write(dir.path(), "docs/lints/lint_a.md", "a");
        write(dir.path(), "docs/lints/lint_b.md", "b");
        write(
            dir.path(),
            "Dockerfile",
            "FROM scratch\n    cargo install cargo-dylint dylint-link --version \"^7.0.0\" --locked && \\\n",
        );
        let msg = parse_err(run_with(Some(&pkg), Some(&out)));
        assert!(msg.contains("pin drift"), "{msg}");
        assert!(
            msg.contains(DYLINT_VERSION_CONSTRAINT) && msg.contains("\"^7.0.0\""),
            "{msg}"
        );
        assert!(msg.contains("Dockerfile:2"), "{msg}");
    }
}
