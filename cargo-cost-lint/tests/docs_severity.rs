use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn workspace_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path
}

#[test]
fn lint_pages_match_declared_severity() {
    let ws = workspace_dir();
    let lib_rs = fs::read_to_string(ws.join("soroban_cost_lints/src/lib.rs"))
        .expect("Failed to read soroban_cost_lints/src/lib.rs");
    let docs_dir = ws.join("docs/lints");
    let mut declared_levels = HashMap::new();
    let lines: Vec<_> = lib_rs.lines().collect();
    let mut index = 0;

    while index < lines.len() {
        if lines[index].trim() != "rustc_session::declare_lint! {" {
            index += 1;
            continue;
        }

        let block_start = index + 1;
        let block_end = lines[block_start..]
            .iter()
            .position(|line| line.trim() == "}")
            .map(|offset| block_start + offset)
            .expect("declare_lint! block must be closed");
        let name = lines[block_start..block_end]
            .iter()
            .find_map(|line| line.trim().strip_prefix("pub "))
            .expect("declare_lint! block must declare a public lint")
            .trim_end_matches(',')
            .to_lowercase();
        let level = lines[block_start..block_end]
            .iter()
            .find_map(|line| {
                let candidate = line.trim().trim_end_matches(',');
                matches!(candidate, "Allow" | "Warn" | "Deny" | "Forbid")
                    .then_some(candidate.to_lowercase())
            })
            .expect("declare_lint! block must declare a lint level");

        declared_levels.insert(name, level);
        index = block_end + 1;
    }

    for (name, expected_level) in declared_levels {
        let path = docs_dir.join(format!("{name}.md"));
        if !path.is_file() {
            continue;
        }
        let page = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("Failed to read {}: {error}", path.display()));
        let Some(actual_level) = page.lines().find_map(|line| {
            line.trim()
                .strip_prefix("**Default Severity:**")
                .map(|value| value.trim().trim_matches('`').to_lowercase())
        }) else {
            continue;
        };

        assert_eq!(
            actual_level,
            *expected_level,
            "Default severity in {} does not match the declared lint level",
            path.display()
        );
    }
}
