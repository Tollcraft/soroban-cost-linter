use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=../soroban_cost_lints/src/lib.rs");
    
    let content = fs::read_to_string("../soroban_cost_lints/src/lib.rs")
        .expect("Failed to read soroban_cost_lints/src/lib.rs");
        
    let start_marker = "lint_store.register_lints(&[";
    let start = content.find(start_marker).expect("Could not find register_lints in lib.rs");
    let content_after = &content[start..];
    let end = content_after.find("]);").expect("Could not find end of register_lints");
    
    let list_str = &content_after[start_marker.len()..end];
    
    let mut names = Vec::new();
    for line in list_str.lines() {
        let trimmed = line.trim().trim_end_matches(',');
        if !trimmed.is_empty() && !trimmed.starts_with("//") {
            names.push(trimmed.to_lowercase());
        }
    }
    
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("lint_names.rs");
    
    let mut out = String::new();
    out.push_str("pub const LINT_NAMES: &[&str] = &[\n");
    for name in names {
        out.push_str(&format!("    \"{}\",\n", name));
    }
    out.push_str("];\n");
    
    fs::write(&dest_path, out).expect("Failed to write lint_names.rs");
}
