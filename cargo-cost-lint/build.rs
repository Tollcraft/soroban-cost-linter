use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::Path;

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
            Error::MissingEnv => write!(f, "OUT_DIR environment variable not set"),
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

/// Parse lint names from the `register_lints` call, returning lowercase names
/// in the order they appear.
fn parse_register_lints(content: &str) -> Result<Vec<String>> {
    let start_marker = "lint_store.register_lints(&[";
    let start = content
        .find(start_marker)
        .ok_or_else(|| Error::Parse("Could not find register_lints in lib.rs".into()))?;
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

/// Parse `declare_lint! { ... }` blocks to extract each lint's name, default
/// level, and one-line description.
///
/// Returns metadata for lints in the order they appear in source.
fn parse_declare_lints(content: &str) -> Vec<LintMeta> {
    let mut results = Vec::new();
    let mut remaining = content;

    while let Some(start) = remaining.find("declare_lint! {") {
        let after_start = &remaining[start + "declare_lint! {".len()..];

        // Find matching closing brace, respecting nested braces.
        let mut depth: u32 = 1;
        let mut end_offset = 0;
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

        if end_offset == 0 {
            // Bail: malformed declare_lint!
            break;
        }

        let block = &after_start[..end_offset];

        // Extract the three key lines from the block body.
        let lines: Vec<&str> = block
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with("//"))
            .collect();

        if lines.len() >= 3 {
            // Line 0: "pub LINT_NAME," -> "lint_name"
            let raw_name = lines[0]
                .trim_start_matches("pub ")
                .trim_end_matches(',')
                .trim();
            let name = raw_name.to_lowercase();

            // Line 1: "Warn," -> "warn"
            let level = lines[1].trim_end_matches(',').to_lowercase();

            // Line 2: '"description"' -> "description"
            let description = lines[2].trim().trim_matches('"').to_string();

            results.push(LintMeta {
                name,
                level,
                description,
            });
        }

        remaining = &after_start[end_offset + 1..];
    }

    results
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

fn run() -> Result<()> {
    println!("cargo:rerun-if-changed=../soroban_cost_lints/src/lib.rs");

    let content = fs::read_to_string("../soroban_cost_lints/src/lib.rs")?;

    let names = parse_register_lints(&content)?;
    let declared = parse_declare_lints(&content);

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
    let docs_dir = "../docs/lints";
    for name in &names {
        let doc_path = format!("{}/{}.md", docs_dir, name);
        assert!(
            Path::new(&doc_path).exists(),
            "lint '{}' is registered but has no doc file at '{}'. \
             Create a documentation page at docs/lints/{}.md to explain \
             what the lint does, why it is expensive, and how to fix it.",
            name,
            doc_path,
            name
        );
    }

    // --- Read each doc file and embed as raw string literals ---
    let mut explanations: Vec<(String, String)> = Vec::new();
    for name in &names {
        let doc_path = format!("{}/{}.md", docs_dir, name);
        let doc_content = fs::read_to_string(&doc_path)
            .unwrap_or_else(|e| panic!("Failed to read doc file '{}': {}", doc_path, e));
        // Notify cargo to re-run build.rs when any doc file changes
        println!("cargo:rerun-if-changed={}", doc_path);
        explanations.push((name.clone(), doc_content));
    }

    let mut declarations = Vec::new();
    let mut search_from = 0usize;
    while let Some(start) = content[search_from..].find("rustc_session::declare_lint! {") {
        let absolute_start = search_from + start;
        let after = &content[absolute_start + "rustc_session::declare_lint! {".len()..];
        let end = after.find('}').expect("Could not parse declare_lint block");
        let block = &after[..end];
        // Filter out comment lines (///, //) and blank lines before splitting
        // by comma, so that doc-commented declare_lint! blocks parse correctly.
        let clean_lines: Vec<&str> = block
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with("//"))
            .collect();
        let block_text = clean_lines.join("\n");
        let parts: Vec<&str> = block_text.split(',').map(str::trim).collect();
        if parts.len() >= 3 {
            let lint_name = parts[0].trim_start_matches("pub ").trim();
            let default_level = parts[1].to_ascii_lowercase();
            let description = parts[2].trim().trim_matches('"').to_string();
            declarations.push((lint_name.to_string(), default_level, description));
        }
        search_from = absolute_start + 1;
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
                    let lint_name = lint_part.split(',').next().unwrap().trim();
                    if let Some(category_part) = entry.split("category:").nth(1) {
                        let category = category_part
                            .split(',')
                            .next()
                            .unwrap()
                            .trim()
                            .split("::")
                            .last()
                            .unwrap();
                        category_map.insert(lint_name.to_lowercase(), category.to_string());
                    }
                }
            }
        }
    }

    let out_dir = env::var_os("OUT_DIR").ok_or(Error::MissingEnv)?;
    let names_path = Path::new(&out_dir).join("lint_names.rs");
    let metadata_path = Path::new(&out_dir).join("lint_metadata.rs");
    let info_path = Path::new(&out_dir).join("lint_info.rs");
    let explanations_path = Path::new(&out_dir).join("lint_explanations.rs");

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
        if let Some((_, default_level, description)) = declarations
            .iter()
            .find(|(lint_name, _, _)| lint_name.to_lowercase() == *name)
        {
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
                rust_string(default_level)
            ));
            metadata_out.push_str(&format!(
                "            description: {},\n",
                rust_string(description)
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
