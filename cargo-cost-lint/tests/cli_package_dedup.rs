#[path = "../src/package_args.rs"]
mod package_args;
use package_args::validate_and_build_package_args;

// ── no packages ───────────────────────────────────────────────────────────────

#[test]
fn empty_packages_returns_empty_args() {
    let result = validate_and_build_package_args(&[], false, &["pkg_a".to_string()]);
    assert_eq!(result, Ok(vec![]));
}

// ── single package ────────────────────────────────────────────────────────────

#[test]
fn single_package_produces_one_flag_pair() {
    let available = vec!["pkg_a".to_string(), "pkg_b".to_string()];
    let result = validate_and_build_package_args(&["pkg_a".to_string()], false, &available);
    assert_eq!(
        result,
        Ok(vec!["--package".to_string(), "pkg_a".to_string()])
    );
}

// ── multiple packages ─────────────────────────────────────────────────────────

#[test]
fn multiple_distinct_packages_all_appear_in_output() {
    let available = vec![
        "pkg_a".to_string(),
        "pkg_b".to_string(),
        "pkg_c".to_string(),
    ];
    let result = validate_and_build_package_args(
        &["pkg_a".to_string(), "pkg_b".to_string()],
        false,
        &available,
    );
    assert_eq!(
        result,
        Ok(vec![
            "--package".to_string(),
            "pkg_a".to_string(),
            "--package".to_string(),
            "pkg_b".to_string(),
        ])
    );
}

// ── deduplication ─────────────────────────────────────────────────────────────

#[test]
fn duplicate_package_is_deduplicated_to_one_flag_pair() {
    let available = vec!["contract_a".to_string()];
    let result = validate_and_build_package_args(
        &["contract_a".to_string(), "contract_a".to_string()],
        false,
        &available,
    );
    // The duplicate must be collapsed: only one --package contract_a pair.
    assert_eq!(
        result,
        Ok(vec!["--package".to_string(), "contract_a".to_string()])
    );
}

#[test]
fn triplicate_package_collapses_to_one_flag_pair() {
    let available = vec!["contract_a".to_string()];
    let result = validate_and_build_package_args(
        &[
            "contract_a".to_string(),
            "contract_a".to_string(),
            "contract_a".to_string(),
        ],
        false,
        &available,
    );
    assert_eq!(
        result,
        Ok(vec!["--package".to_string(), "contract_a".to_string()])
    );
}

#[test]
fn mixed_duplicate_and_distinct_preserves_first_seen_order() {
    let available = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let result = validate_and_build_package_args(
        &[
            "b".to_string(),
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "a".to_string(),
        ],
        false,
        &available,
    );
    // Expected: b, a, c in first-seen order; second occurrences dropped.
    assert_eq!(
        result,
        Ok(vec![
            "--package".to_string(),
            "b".to_string(),
            "--package".to_string(),
            "a".to_string(),
            "--package".to_string(),
            "c".to_string(),
        ])
    );
}

#[test]
fn all_duplicates_produces_single_entry_per_name() {
    let available = vec!["x".to_string(), "y".to_string()];
    let result = validate_and_build_package_args(
        &[
            "x".to_string(),
            "y".to_string(),
            "x".to_string(),
            "y".to_string(),
        ],
        false,
        &available,
    );
    assert_eq!(
        result,
        Ok(vec![
            "--package".to_string(),
            "x".to_string(),
            "--package".to_string(),
            "y".to_string(),
        ])
    );
}

// ── workspace flag ────────────────────────────────────────────────────────────

#[test]
fn workspace_flag_returns_workspace_arg() {
    let result = validate_and_build_package_args(&[], true, &["pkg_a".to_string()]);
    assert_eq!(result, Ok(vec!["--workspace".to_string()]));
}

#[test]
fn workspace_and_packages_together_is_an_error() {
    let available = vec!["pkg_a".to_string()];
    let result = validate_and_build_package_args(&["pkg_a".to_string()], true, &available);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("--workspace"));
}

// ── unknown package ───────────────────────────────────────────────────────────

#[test]
fn unknown_package_name_returns_error() {
    let available = vec!["pkg_a".to_string()];
    let result = validate_and_build_package_args(&["unknown_crate".to_string()], false, &available);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("unknown_crate"),
        "error should name the bad package"
    );
    assert!(err.contains("pkg_a"), "error should list valid members");
}
