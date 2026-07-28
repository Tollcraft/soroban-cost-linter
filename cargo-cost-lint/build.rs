use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

fn rust_string(value: &str) -> String {
    format!("{:?}", value)
}

fn main() {
    println!("cargo:rerun-if-changed=../soroban_cost_lints/src/lib.rs");

    let content = fs::read_to_string("../soroban_cost_lints/src/lib.rs")
        .expect("Failed to read soroban_cost_lints/src/lib.rs");

/// Parse lint names from the `register_lints` call, returning lowercase names
/// in the order they appear.
fn parse_register_lints(content: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let start_marker = "lint_store.register_lints(&[";
    let start = content
        .find(start_marker)
        .ok_or("Could not find register_lints in lib.rs")?;
    let content_after = &content[start..];
    let end = content_after
        .find("]);")
        .ok_or("Could not find end of register_lints")?;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../soroban_cost_lints/src/lib.rs");

    let content = fs::read_to_string("../soroban_cost_lints/src/lib.rs")
        .map_err(|e| format!("Failed to read soroban_cost_lints/src/lib.rs: {}", e))?;

    let names = parse_register_lints(&content)?;
    let declared = parse_declare_lints(&content);

    // Build a name\u2192metadata lookup from the declare_lint! blocks.
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

    let out_dir = env::var_os("OUT_DIR").ok_or("OUT_DIR environment variable not set")?;
    let names_path = Path::new(&out_dir).join("lint_names.rs");
    let metadata_path = Path::new(&out_dir).join("lint_metadata.rs");

    let mut out = String::new();

    // Emit LINT_NAMES (used by the filter logic in main.rs).
    out.push_str("pub const LINT_NAMES: &[&str] = &[\n");
    for name in &names {
        if let Some((_, default_level, description)) = declarations.iter().find(|(lint_name, _, _)| lint_name.to_lowercase() == *name) {
            let category = category_map.get(name).map(|value| value.as_str()).unwrap_or("Unknown");
            let docs_path = format!("https://github.com/Tollcraft/soroban-cost-linter/blob/main/docs/lints/{}.md", name);
            metadata_out.push_str("        LintInventoryEntry {\n");
            metadata_out.push_str(&format!("            name: {},\n", rust_string(name)));
            metadata_out.push_str(&format!("            default_level: {},\n", rust_string(default_level)));
            metadata_out.push_str(&format!("            description: {},\n", rust_string(description)));
            metadata_out.push_str(&format!("            category: {},\n", rust_string(category)));
            metadata_out.push_str(&format!("            documentation_url: {},\n", rust_string(&docs_path)));
            metadata_out.push_str("        },\n");
        }
    }

    fs::write(&names_path, out).map_err(|e| format!("Failed to write lint_names.rs: {}", e))?;

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
    for lint in &ordered {
        let cat = category_map
            .get(&lint.name)
            .map(|c| c.as_str())
            .unwrap_or("Unknown");
        metadata_out.push_str(&format!(
            "        LintInventoryEntry {{ name: \"{}\", default_level: \"{}\", description: \"{}\", category: \"{}\", documentation_url: \"\" }},\n",
            lint.name, lint.level, lint.description, cat
        ));
    }
    metadata_out.push_str("    ],\n");
    metadata_out.push_str("};\n");

    fs::write(&metadata_path, metadata_out)
        .map_err(|e| format!("Failed to write lint_metadata.rs: {}", e))?;

    Ok(())
}
