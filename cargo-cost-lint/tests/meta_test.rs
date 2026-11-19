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

    // The README uses text like "Thirty lints ship in", "Thirty-one", or
    // "30 lints ship in". The word is derived from the counted registry rather
    // than hard-coded per count, so registering the next lint does not redden
    // this test.
    let word = count_word(count);

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

/// The English word for `count` in the "Forty-one" spelling the README uses,
/// derived from the number instead of a table that has to grow with the lint
/// registry.
fn count_word(count: usize) -> String {
    const ONES: [&str; 20] = [
        "Zero",
        "One",
        "Two",
        "Three",
        "Four",
        "Five",
        "Six",
        "Seven",
        "Eight",
        "Nine",
        "Ten",
        "Eleven",
        "Twelve",
        "Thirteen",
        "Fourteen",
        "Fifteen",
        "Sixteen",
        "Seventeen",
        "Eighteen",
        "Nineteen",
    ];
    const TENS: [&str; 10] = [
        "", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety",
    ];

    if let Some(ones) = ONES.get(count) {
        return (*ones).to_string();
    }
    let tens = count / 10;
    let rest = count % 10;
    let Some(tens_word) = TENS.get(tens).filter(|word| !word.is_empty()) else {
        return "unknown".to_string();
    };
    if rest == 0 {
        (*tens_word).to_string()
    } else {
        format!("{}-{}", tens_word, ONES[rest].to_lowercase())
    }
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

    // Check deny count: "exactly two ... default to deny"
    assert_eq!(
        deny_count, 2,
        "severity_rationale says 'exactly two default to deny' but registry has {} Deny lints",
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
