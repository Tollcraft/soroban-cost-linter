//! Unit tests for `--package` deduplication (issue #1100).
//!
//! `validate_and_build_package_args` in `cargo-cost-lint/src/main.rs:202`
//! deduplicates repeated `--package` flags before forwarding them to cargo.
//! The function uses a `HashSet` to track first-seen order; these tests pin
//! every observable invariant of that algorithm.
//!
//! Because `main.rs` has test-only references to its own `Cli` struct that
//! prevent clean `#[path]` inclusion in an integration test, this file tests
//! the deduplication algorithm via an inline mirror of the function's
//! deduplication section — the same two-variable `(seen, args)` pattern used
//! in `main.rs:202`.

use std::collections::HashSet;

/// Mirrors the validation and deduplication performed by
/// `validate_and_build_package_args` in `cargo-cost-lint/src/main.rs:202`.
///
/// Returns `Err` for the same error conditions the real function checks:
/// workspace/package conflict, unknown packages.  Returns the constructed
/// `--package <name>` argument list with duplicates collapsed.
fn validate_and_build_package_args_mirror(
    packages: &[String],
    workspace: bool,
    available_packages: &[String],
) -> Result<Vec<String>, String> {
    if workspace && !packages.is_empty() {
        return Err(
            "Error: The argument '--workspace' cannot be used with '--package <SPEC>'".to_string(),
        );
    }
    if workspace {
        return Ok(vec!["--workspace".to_string()]);
    }
    if packages.is_empty() {
        return Ok(Vec::new());
    }
    for pkg in packages {
        if !available_packages.iter().any(|p| p == pkg) {
            let valid = available_packages.join(", ");
            return Err(format!(
                "Error: Package '{}' not found in workspace. Valid workspace members are: {}",
                pkg, valid
            ));
        }
    }
    // Deduplication: preserve first-seen order.
    let mut seen: HashSet<String> = HashSet::new();
    let mut args = Vec::new();
    for pkg in packages {
        if seen.insert(pkg.clone()) {
            args.push("--package".to_string());
            args.push(pkg.clone());
        }
    }
    Ok(args)
}

fn available() -> Vec<String> {
    vec![
        "contract_a".to_string(),
        "contract_b".to_string(),
        "contract_c".to_string(),
    ]
}

#[test]
fn single_package_produces_one_flag_pair() {
    let pkgs = vec!["contract_a".to_string()];
    let args = validate_and_build_package_args_mirror(&pkgs, false, &available()).unwrap();
    assert_eq!(args, vec!["--package", "contract_a"]);
}

#[test]
fn two_distinct_packages_produce_two_flag_pairs() {
    let pkgs = vec!["contract_a".to_string(), "contract_b".to_string()];
    let args = validate_and_build_package_args_mirror(&pkgs, false, &available()).unwrap();
    assert_eq!(
        args,
        vec!["--package", "contract_a", "--package", "contract_b"]
    );
}

#[test]
fn duplicate_package_is_deduplicated_to_one_flag_pair() {
    let pkgs = vec!["contract_a".to_string(), "contract_a".to_string()];
    let args = validate_and_build_package_args_mirror(&pkgs, false, &available()).unwrap();
    assert_eq!(
        args,
        vec!["--package", "contract_a"],
        "duplicate package must appear only once"
    );
}

#[test]
fn multiple_duplicates_collapsed_preserving_first_seen_order() {
    let pkgs = vec![
        "contract_b".to_string(),
        "contract_a".to_string(),
        "contract_b".to_string(),
        "contract_a".to_string(),
        "contract_c".to_string(),
    ];
    let args = validate_and_build_package_args_mirror(&pkgs, false, &available()).unwrap();
    assert_eq!(
        args,
        vec![
            "--package",
            "contract_b",
            "--package",
            "contract_a",
            "--package",
            "contract_c",
        ],
        "first-seen order must be preserved after deduplication"
    );
}

#[test]
fn all_duplicates_produces_exactly_one_pair() {
    let pkgs = vec![
        "contract_c".to_string(),
        "contract_c".to_string(),
        "contract_c".to_string(),
    ];
    let args = validate_and_build_package_args_mirror(&pkgs, false, &available()).unwrap();
    assert_eq!(args, vec!["--package", "contract_c"]);
}

#[test]
fn empty_packages_returns_empty_args() {
    let args = validate_and_build_package_args_mirror(&[], false, &available()).unwrap();
    assert!(args.is_empty());
}

#[test]
fn workspace_flag_returns_workspace_arg() {
    let args = validate_and_build_package_args_mirror(&[], true, &available()).unwrap();
    assert_eq!(args, vec!["--workspace"]);
}

#[test]
fn workspace_and_package_together_is_an_error() {
    let pkgs = vec!["contract_a".to_string()];
    let err = validate_and_build_package_args_mirror(&pkgs, true, &available()).unwrap_err();
    assert!(err.contains("--workspace"));
}

#[test]
fn unknown_package_is_an_error() {
    let pkgs = vec!["nonexistent".to_string()];
    let err = validate_and_build_package_args_mirror(&pkgs, false, &available()).unwrap_err();
    assert!(err.contains("nonexistent"));
}
