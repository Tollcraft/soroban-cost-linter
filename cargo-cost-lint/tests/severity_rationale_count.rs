//! Guards the hand-typed lint totals in `docs/severity_rationale.md` against
//! drift: the shipped count comes from `register_lints`, and the `deny` count
//! from the `declare_lint!` blocks of the registered lints.

use std::fs;
use std::path::PathBuf;

fn workspace_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path
}

/// Lint constants listed in `lint_store.register_lints(&[ ... ]);`.
fn registered_lints(lib_rs: &str) -> Vec<String> {
    let start_marker = "lint_store.register_lints(&[";
    let start = lib_rs
        .find(start_marker)
        .expect("register_lints must exist in lib.rs");
    let content_after = &lib_rs[start + start_marker.len()..];
    let end = content_after
        .find("]);")
        .expect("end of register_lints must exist");
    content_after[..end]
        .lines()
        .map(|l| l.trim().trim_end_matches(',').to_string())
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .collect()
}

/// Default level (`Allow`/`Warn`/`Deny`) of the `declare_lint!` block for `name`.
fn declared_level<'a>(lib_rs: &'a str, name: &str) -> &'a str {
    let needle = format!("pub {name},");
    let at = lib_rs
        .find(&needle)
        .unwrap_or_else(|| panic!("no declare_lint! block for registered lint {name}"));
    lib_rs[at + needle.len()..]
        .split(',')
        .next()
        .expect("declare_lint! block must have a level")
        .trim()
}

#[test]
fn severity_rationale_lint_totals_match_registry() {
    let ws = workspace_dir();
    let lib_rs = fs::read_to_string(ws.join("soroban_cost_lints/src/lib.rs"))
        .expect("Failed to read lib.rs");
    let doc = fs::read_to_string(ws.join("docs/severity_rationale.md"))
        .expect("Failed to read docs/severity_rationale.md");

    let lints = registered_lints(&lib_rs);
    let total = lints.len();
    let deny = lints
        .iter()
        .filter(|name| declared_level(&lib_rs, name) == "Deny")
        .count();
    let warn = lints
        .iter()
        .filter(|name| declared_level(&lib_rs, name) == "Warn")
        .count();

    assert_eq!(
        deny, 1,
        "docs/severity_rationale.md explains a single deny-by-default lint; \
         the registry now has {deny}"
    );

    let expected = format!(
        "This linter ships **{total} lints**: **{warn}** default to `warn`, and exactly one"
    );
    assert!(
        doc.contains(&expected),
        "docs/severity_rationale.md lint totals are stale: expected the sentence \
         to start with {expected:?} ({total} registered, {warn} warn, {deny} deny)"
    );
}
