//! Integration tests for `--diff-only` path matching, covering the fix for
//! issue #1095: `is_file_changed` compares full repository-relative paths so
//! that editing `package_a/src/mod.rs` never un-filters a finding that is
//! anchored to `package_b/src/mod.rs` (or any other package's file with the
//! same basename).

// Minimal stubs required by diff_files.rs when compiled in isolation.
#[allow(dead_code)]
mod error {
    #[derive(Debug)]
    pub enum LinterError {
        Other(String),
    }

    impl std::fmt::Display for LinterError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                LinterError::Other(s) => write!(f, "{s}"),
            }
        }
    }

    pub type LinterResult<T> = Result<T, LinterError>;
}

#[path = "../src/diff_files.rs"]
#[allow(dead_code)]
mod diff_files;

use diff_files::is_file_changed;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from("/workspace")
}

// ---------------------------------------------------------------------------
// Cross-package same-basename matching is forbidden (#1095)
// ---------------------------------------------------------------------------

#[test]
fn same_basename_in_different_subtree_does_not_match() {
    // Editing package_a/src/mod.rs must NOT make is_file_changed return true
    // for package_b/src/mod.rs.
    let changed = vec!["package_a/src/mod.rs".to_string()];
    assert!(
        !is_file_changed("package_b/src/mod.rs", &changed, &root()),
        "package_b/src/mod.rs must not match a change to package_a/src/mod.rs"
    );
}

#[test]
fn exact_path_match_is_accepted() {
    let changed = vec!["package_a/src/mod.rs".to_string()];
    assert!(
        is_file_changed("package_a/src/mod.rs", &changed, &root()),
        "exact path must match"
    );
}

#[test]
fn absolute_path_matching_own_package_resolves_correctly() {
    // When the finding's file field is absolute (absolute path under /workspace),
    // it should still match the repository-relative changed entry.
    let changed = vec!["package_b/src/lib.rs".to_string()];
    assert!(
        is_file_changed("/workspace/package_b/src/lib.rs", &changed, &root()),
        "absolute form of the same file must match its relative changed entry"
    );
}

#[test]
fn absolute_path_in_different_package_does_not_match() {
    let changed = vec!["package_a/src/lib.rs".to_string()];
    assert!(
        !is_file_changed("/workspace/package_b/src/lib.rs", &changed, &root()),
        "absolute path for package_b must not match a change to package_a"
    );
}

// ---------------------------------------------------------------------------
// Deep subtree isolation
// ---------------------------------------------------------------------------

#[test]
fn nested_subtrees_are_isolated() {
    // contracts/alpha/src/token.rs vs contracts/beta/src/token.rs
    let changed = vec!["contracts/alpha/src/token.rs".to_string()];
    assert!(
        !is_file_changed("contracts/beta/src/token.rs", &changed, &root()),
        "token.rs in beta must not match a change to alpha's token.rs"
    );
    assert!(
        is_file_changed("contracts/alpha/src/token.rs", &changed, &root()),
        "token.rs in alpha must match the change"
    );
}

#[test]
fn multiple_changed_files_only_match_exact_paths() {
    let changed = vec![
        "package_a/src/lib.rs".to_string(),
        "package_b/src/main.rs".to_string(),
    ];
    // A file not in the changed list must not match.
    assert!(
        !is_file_changed("package_c/src/lib.rs", &changed, &root()),
        "package_c must not match changes in package_a and package_b"
    );
    // Each changed file matches itself.
    assert!(is_file_changed("package_a/src/lib.rs", &changed, &root()));
    assert!(is_file_changed("package_b/src/main.rs", &changed, &root()));
}

// ---------------------------------------------------------------------------
// Path normalisation
// ---------------------------------------------------------------------------

#[test]
fn backslash_separators_normalise_to_forward_slash() {
    let changed = vec!["package_b/src/lib.rs".to_string()];
    assert!(
        is_file_changed(r"package_b\src\lib.rs", &changed, &root()),
        "Windows-style backslash separators must normalise to forward slashes"
    );
}

#[test]
fn dotdot_segments_are_resolved() {
    let changed = vec!["package_b/src/lib.rs".to_string()];
    assert!(
        is_file_changed("package_b/../package_b/src/lib.rs", &changed, &root()),
        "intra-path .. must be resolved before comparison"
    );
}

#[test]
fn empty_changed_list_never_matches() {
    let changed: Vec<String> = vec![];
    assert!(
        !is_file_changed("src/lib.rs", &changed, &root()),
        "nothing should match an empty changed list"
    );
}
