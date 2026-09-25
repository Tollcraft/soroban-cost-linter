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
/// Returns metadata for lints in the order they appear in source.
fn parse_declare_lints(content: &str) -> Result<Vec<LintMeta>> {
    let mut results = Vec::new();
    let mut search_from = 0;

    while let Some(rel_start) = content[search_from..].find("declare_lint! {") {
        let absolute_start = search_from + rel_start;
        let after_start = &content[absolute_start + "declare_lint! {".len()..];
        let start_line = content[..absolute_start].lines().count() + 1;

        // Find matching closing brace, respecting nested braces.
        let mut depth: u32 = 1;
        let mut end_offset = None;
        for (i, ch) in after_start.char_indices() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    end_offset = Some(i);
                    break;
                }
            }
        }

        let end_idx = end_offset.ok_or_else(|| {
            Error::Parse(format!(
                "Unclosed declare_lint! block starting at line {}",
                start_line
            ))
        })?;

        let block = &after_start[..end_idx];

        // Extract non-comment, non-empty lines from the block body.
        let lines: Vec<&str> = block
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with("#["))
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

        search_from = absolute_start + "declare_lint! {".len() + end_idx + 1;
    }

    Ok(results)
}

/// Parse the `LINT_METADATA` registry into a lowercase-name → category map.
///
/// This is the third and final view of `lib.rs` that build.rs needs. It is
/// keyed the same way as the `declare_lint!` blocks, so `run()` can require the
/// three views to agree instead of letting a lint quietly lose its category.
fn parse_lint_metadata_categories(content: &str) -> Result<HashMap<String, String>> {
    const MARKER: &str = "pub const LINT_METADATA";
    let marker_start = content
        .find(MARKER)
        .ok_or_else(|| Error::Parse("lib.rs has no `pub const LINT_METADATA` registry".into()))?;

    let rel = content[marker_start..].find("= &[").ok_or_else(|| {
        Error::Parse("`LINT_METADATA` in lib.rs is not a slice literal (`= &[ ... ]`)".into())
    })?;
    // Index of the `[` that opens the slice literal.
    let open = marker_start + rel + "= &[".len() - 1;
    let close = closing_bracket(content, open)?;
    let body = &content[open + 1..close];

    let mut categories = HashMap::new();
    for entry in body.split("LintMeta {").skip(1) {
        let mut name = None;
        let mut category = None;
        for line in entry.lines() {
            let line = line.trim().trim_end_matches(',');
            if let Some(value) = line.strip_prefix("name:") {
                name = Some(value.trim().trim_matches('"').to_string());
            } else if let Some(value) = line.strip_prefix("category:") {
                let value = value.trim();
                // `LintCategory::Compute` -> `Compute`
                category = Some(value.rsplit("::").next().unwrap_or(value).to_string());
            }
        }
        let (name, category) = match (name, category) {
            (Some(name), Some(category)) => (name, category),
            _ => {
                return Err(Error::Parse(format!(
                    "Could not parse a LINT_METADATA entry (expected `name:` and `category:` \
                     fields): {}",
                    entry.trim()
                )));
            }
        };
        if name.is_empty() || category.is_empty() {
            return Err(Error::Parse(format!(
                "LINT_METADATA entry has an empty name or category: {}",
                entry.trim()
            )));
        }
        if categories.insert(name.to_lowercase(), category).is_some() {
            return Err(Error::Parse(format!(
                "duplicate LINT_METADATA row for lint '{}'",
                name
            )));
        }
    }

    if categories.is_empty() {
        return Err(Error::Parse(
            "LINT_METADATA in lib.rs contains no entries".into(),
        ));
    }
    Ok(categories)
}

/// Index of the `]` closing the `[` at `open`, skipping brackets that appear
/// inside string literals.
fn closing_bracket(content: &str, open: usize) -> Result<usize> {
    let mut depth: u32 = 0;
    let mut in_string = false;
    let mut escaped = false;
    for (i, ch) in content[open..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(open + i);
                }
            }
            _ => {}
        }
    }
    Err(Error::Parse(format!(
        "Unterminated `[` in lib.rs at byte {}",
        open
    )))
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
    // the `declare_lint!` blocks and `parse_lint_metadata_categories` the
    // `LINT_METADATA` registry. They are independent textual parsers, so every
    // lint must show up in all three or the build fails here rather than
    // shipping an inventory with a silent gap in it.
    let names = parse_register_lints(&content)?;
    let declared = parse_declare_lints(&content)?;
    let categories = parse_lint_metadata_categories(&content)?;

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
        .filter(|name| !categories.contains_key(*name))
        .collect();
    if !missing_category.is_empty() {
        missing_category.sort_unstable();
        return Err(Error::Parse(format!(
            "lint(s) {} are registered in lib.rs but have no LINT_METADATA row, so the \
             inventory would carry no category",
            join_quoted(&missing_category)
        )));
    }

    let mut orphan_category: Vec<&str> = categories
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
        let category = categories.get(name).ok_or_else(|| {
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
