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

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    println!("cargo:rerun-if-changed=../soroban_cost_lints/src/lib.rs");

    let content = fs::read_to_string("../soroban_cost_lints/src/lib.rs")?;

/// Parse lint names from the `register_lints` call, returning lowercase names
/// in the order they appear.
fn parse_register_lints(content: &str) -> Vec<String> {
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
    names
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
    println!("cargo:rerun-if-changed=../soroban_cost_lints/src/lib.rs");

    let content = fs::read_to_string("../soroban_cost_lints/src/lib.rs")
        .expect("Failed to read soroban_cost_lints/src/lib.rs");

    let names = parse_register_lints(&content);
    let declared = parse_declare_lints(&content);

    // Build a name→metadata lookup from the declare_lint! blocks.
    let metadata_by_name: std::collections::HashMap<&str, &LintMeta> =
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
        let parts: Vec<&str> = block.split(',').map(str::trim).collect();
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

    let mut declarations = Vec::new();
    let mut search_from = 0usize;
    while let Some(start) = content[search_from..].find("rustc_session::declare_lint! {") {
        let absolute_start = search_from + start;
        let after = &content[absolute_start + "rustc_session::declare_lint! {".len()..];
        let end = after.find('}').expect("Could not parse declare_lint block");
        let block = &after[..end];
        let parts: Vec<&str> = block.split(',').map(str::trim).collect();
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
    let dest_path = Path::new(&out_dir).join("lint_names.rs");

    let mut names_out = String::new();
    names_out.push_str("pub const LINT_NAMES: &[&str] = &[\n");
    for name in &names {
        names_out.push_str(&format!("    \"{}\",\n", name));
    }
    names_out.push_str("];\n");
    fs::write(&names_path, names_out).expect("Failed to write lint_names.rs");

    fs::write(&dest_path, out)?;

    Ok(())
}
