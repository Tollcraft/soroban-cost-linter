use std::fs;
use std::path::PathBuf;

fn workspace_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path
}

#[test]
fn test_github_action_drift() {
    let ws = workspace_dir();
    let action = fs::read_to_string(ws.join("action.yml")).unwrap();
    let template = fs::read_to_string(ws.join("templates/github-action.yml")).unwrap();

    let action_toolchain = action
        .lines()
        .find(|l| l.contains("default: 'nightly"))
        .unwrap()
        .split('\'')
        .nth(1)
        .unwrap();
    let template_toolchain = template
        .lines()
        .find(|l| l.contains("toolchain: nightly-"))
        .unwrap()
        .split_whitespace()
        .last()
        .unwrap();

    assert_eq!(
        action_toolchain, template_toolchain,
        "Toolchains in action.yml and template drift!"
    );
}

#[test]
fn test_readme_lint_count() {
    let ws = workspace_dir();
    let readme = fs::read_to_string(ws.join("README.md")).expect("Failed to read README.md");
    let lib_rs = fs::read_to_string(ws.join("soroban_cost_lints/src/lib.rs"))
        .expect("Failed to read lib.rs");

    let start_marker = "lint_store.register_lints(&[";
    let start = lib_rs
        .find(start_marker)
        .expect("register_lints must exist in lib.rs");
    let content_after = &lib_rs[start + start_marker.len()..];
    let end = content_after
        .find("]);")
        .expect("end of register_lints must exist");

    let count = content_after[..end]
        .lines()
        .map(|l| l.trim().trim_end_matches(',').to_lowercase())
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .count();

    // The README uses text like "Thirty lints ship in", "Thirty-one", or "30 lints ship in".
    let word = match count {
        30 => "Thirty",
        31 => "Thirty-one",
        32 => "Thirty-two",
        33 => "Thirty-three",
        34 => "Thirty-four",
        35 => "Thirty-five",
        36 => "Thirty-six",
        37 => "Thirty-seven",
        38 => "Thirty-eight",
        39 => "Thirty-nine",
        40 => "Forty",
        _ => "unknown",
    };

    let expected_num = format!("{} lints ship in", count);
    let expected_word = format!("{} lints ship in", word);

    assert!(
        readme.contains(&expected_num)
            || readme.contains(&expected_word)
            || readme.contains(&expected_word.to_lowercase()),
        "README lint count {} ({}) doesn't match shipped lints!",
        count,
        word
    );
}

#[test]
fn test_severity_rationale_lint_count() {
    let ws = workspace_dir();
    let doc = fs::read_to_string(ws.join("docs/severity_rationale.md"))
        .expect("Failed to read docs/severity_rationale.md");
    let lib_rs = fs::read_to_string(ws.join("soroban_cost_lints/src/lib.rs"))
        .expect("Failed to read lib.rs");

    // Count declare_lint! blocks and their levels
    let total = lib_rs.matches("declare_lint! {").count();
    let deny_count = lib_rs
        .lines()
        .filter(|l| {
            let t = l.trim();
            t == "Deny,"
        })
        .count();
    let warn_count = lib_rs
        .lines()
        .filter(|l| {
            let t = l.trim();
            t == "Warn,"
        })
        .count();

    // The doc may state the count as a numeral ("**40 lints**") or a word
    // ("**Forty lints**"). Accept either form.
    let expected_num = format!("ships **{} lints**", total);
    assert!(
        doc.contains(&expected_num),
        "severity_rationale.md lint count doesn't match registry! Expected '{}', total declared: {}",
        expected_num,
        total
    );

    // Check deny count: "Exactly one ... defaults to deny"
    assert_eq!(
        deny_count, 1,
        "severity_rationale says 'Exactly one defaults to deny' but registry has {} Deny lints",
        deny_count
    );

    // Check warn count consistency
    assert_eq!(
        warn_count + deny_count,
        total,
        "Warn({}) + Deny({}) != total({})",
        warn_count,
        deny_count,
        total
    );
}
