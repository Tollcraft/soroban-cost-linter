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

    let start_marker = "lint_store.register_lints(&[";
    let start = content
        .find(start_marker)
        .expect("Could not find register_lints in lib.rs");
    let content_after = &content[start..];
    let end = content_after
        .find("]);")
        .expect("Could not find end of register_lints");

    let list_str = &content_after[start_marker.len()..end];

    let mut names = Vec::new();
    for line in list_str.lines() {
        let trimmed = line.trim().trim_end_matches(',');
        if !trimmed.is_empty() && !trimmed.starts_with("//") {
            names.push(trimmed.to_lowercase());
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

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let names_path = Path::new(&out_dir).join("lint_names.rs");
    let metadata_path = Path::new(&out_dir).join("lint_metadata.rs");

    let mut names_out = String::new();
    names_out.push_str("pub const LINT_NAMES: &[&str] = &[\n");
    for name in &names {
        names_out.push_str(&format!("    \"{}\",\n", name));
    }
    names_out.push_str("];\n");
    fs::write(&names_path, names_out).expect("Failed to write lint_names.rs");

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

    metadata_out.push_str("    ],\n");
    metadata_out.push_str("};\n");
    fs::write(&metadata_path, metadata_out).expect("Failed to write lint_metadata.rs");
}
