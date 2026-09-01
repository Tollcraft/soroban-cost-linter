use std::collections::HashMap;
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

/// Parse lint names from the registration pattern, returning lowercase names
/// in the order they appear.
///
/// Supports both the legacy `lint_store.register_lints(&[...])` pattern
/// and the current `dylint_lint_impl! { ..., [...] }` macro invocation.
fn parse_register_lints(content: &str) -> Result<Vec<String>> {
    // Try dylint_lint_impl! first (current pattern)
    if let Some(result) = parse_dylint_impl(content) {
        return Ok(result);
    }
    // Fall back to legacy register_lints pattern
    let start_marker = "lint_store.register_lints(&[";
    let start = content.find(start_marker).ok_or_else(|| {
        Error::Parse("Could not find register_lints or dylint_lint_impl in lib.rs".into())
    })?;
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
    Ok(names)
}

/// Try to parse lint names from a `dylint_lint_impl!` macro invocation.
fn parse_dylint_impl(content: &str) -> Option<Vec<String>> {
    let marker = "dylint_lint_impl!";
    let start = content.find(marker)?;
    let after = &content[start..];
    // Find the outermost bracket pair: the lint list is the second argument
    let open = after.find('[')? + 1;
    let close = after[open..].find(']')? + open;
    let list_str = &after[open..close];

    let mut names = Vec::new();
    for line in list_str.lines() {
        let trimmed = line.trim().trim_end_matches(',');
        if !trimmed.is_empty() && !trimmed.starts_with("//") {
            names.push(trimmed.to_lowercase());
        }
    }
    Some(names)
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
    // cargo-cost-lint is an internal workspace tool that embeds compile-time metadata,
    // documentation explanations, and the pinned toolchain version directly from sibling
    // workspace paths (`../soroban_cost_lints`, `../docs/lints`, and `../rust-toolchain`).
    // It is not published independently to crates.io (`publish = false` in Cargo.toml).
    let manifest_dir_str = env::var("CARGO_MANIFEST_DIR").map_err(|_| Error::MissingEnv)?;
    let manifest_dir = PathBuf::from(manifest_dir_str);

    let lib_rs_path = manifest_dir.join("../soroban_cost_lints/src/lib.rs");
    let docs_dir = manifest_dir.join("../docs/lints");
    let toolchain_path = manifest_dir.join("../rust-toolchain");

    println!("cargo:rerun-if-changed=../soroban_cost_lints/src/lib.rs");
    println!("cargo:rerun-if-changed=../docs/lints");
    println!("cargo:rerun-if-changed=../rust-toolchain");

    if !lib_rs_path.exists() {
        return Err(Error::Parse(
            "cargo-cost-lint cannot be built outside the soroban-cost-linter workspace \
             because it relies on compile-time metadata from sibling workspace crates. \
             This crate is not intended for standalone publishing (publish = false)."
                .to_string(),
        ));
    }

    let content = fs::read_to_string(&lib_rs_path).map_err(|e| {
        Error::Parse(format!(
            "Failed to read source file {}: {}",
            lib_rs_path.display(),
            e
        ))
    })?;

    let names = parse_register_lints(&content)?;
    let declared = parse_declare_lints(&content)?;

    // Build a name→metadata lookup from the declare_lint! blocks.
    let metadata_by_name: HashMap<&str, &LintMeta> =
        declared.iter().map(|m| (m.name.as_str(), m)).collect();

    // Derive LINT_INFO in the same order as register_lints, so the two
    // lists can never drift. If a lint is in register_lints but missing
    // a declare_lint! block, we panic at build time.
    let ordered: Vec<&LintMeta> = names
        .iter()
        .map(|name| {
            metadata_by_name
                .get(name.as_str())
                .copied()
                .unwrap_or_else(|| {
                    panic!(
                        "lint '{}' found in register_lints but not in any declare_lint! block",
                        name
                    )
                })
        })
        .collect();

    // Cross-check: every declare_lint! must also appear in register_lints.
    for meta in &declared {
        if !names.contains(&meta.name) {
            panic!(
                "lint '{}' has a declare_lint! block but is not in register_lints",
                meta.name
            );
        }
    }

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
        println!("cargo:rerun-if-changed=../docs/lints/{}.md", name);
        explanations.push((name.clone(), doc_content));
    }

    let mut category_map = HashMap::new();
    let metadata_marker = "pub const LINT_METADATA: &[LintMetadata] = &[";
    if let Some(start) = content.find(metadata_marker) {
        let after = &content[start + metadata_marker.len()..];
        if let Some(end) = after.find("];") {
            let metadata_body = &after[..end];
            for entry in metadata_body.split("LintMetadata {") {
                let entry = entry.trim();
                if entry.is_empty() {
                    continue;
                }
                if let Some(lint_part) = entry.split("lint:").nth(1) {
                    let lint_name = lint_part
                        .split(',')
                        .next()
                        .unwrap_or_else(|| {
                            panic!(
                                "Could not parse lint_name in LINT_METADATA entry: {}",
                                entry
                            )
                        })
                        .trim();
                    if let Some(category_part) = entry.split("category:").nth(1) {
                        let category = category_part
                            .split(',')
                            .next()
                            .unwrap_or_else(|| {
                                panic!(
                                    "Could not parse category_part in LINT_METADATA entry: {}",
                                    entry
                                )
                            })
                            .trim()
                            .split("::")
                            .last()
                            .unwrap_or_else(|| {
                                panic!("Could not extract category name from: {}", category_part)
                            });
                        category_map.insert(lint_name.to_lowercase(), category.to_string());
                    }
                }
            }
        }
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
        if let Some(meta) = metadata_by_name.get(name.as_str()) {
            let category = category_map
                .get(name)
                .map(|value| value.as_str())
                .unwrap_or("Unknown");
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
        } else {
            return Err(Error::Parse(format!(
                "Lint '{}' registered in register_lints but metadata not found in declare_lint! blocks",
                name
            )));
        }
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
    explanations_out.push_str("pub struct LintExplanation {\n");
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
