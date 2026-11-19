//! Verify that `detect_default_branch` falls back to local `refs/heads/`
//! branches when no origin remote is present (#1096).

use std::path::PathBuf;

fn crate_src_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src");
    path
}

fn diff_files_source() -> String {
    let path = crate_src_dir().join("diff_files.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read diff_files.rs: {e}"))
}

/// The fallback to local refs must be present so that `--diff-only` works in
/// repos that have no `origin` remote (e.g. a freshly-cloned mirror or a
/// self-contained local repo).
#[test]
fn detect_default_branch_checks_local_refs_heads_main() {
    let src = diff_files_source();
    assert!(
        src.contains("refs/heads/main"),
        "detect_default_branch must check `refs/heads/main` as a local-branch fallback \
         for repos without an origin remote (issue #1096)"
    );
}

#[test]
fn detect_default_branch_checks_local_refs_heads_master() {
    let src = diff_files_source();
    assert!(
        src.contains("refs/heads/master"),
        "detect_default_branch must check `refs/heads/master` as a local-branch fallback \
         for repos without an origin remote (issue #1096)"
    );
}

/// The error message must inform users that a local branch is a valid option.
#[test]
fn error_message_mentions_local_branch_option() {
    let src = diff_files_source();
    // After the fix the error mentions "local" or "refs/heads"
    let mentions_local = src.contains("local 'main'") || src.contains("refs/heads");
    assert!(
        mentions_local,
        "the 'Could not determine the default branch' error message should tell users \
         about the local branch fallback (issue #1096)"
    );
}

/// `find_merge_base` must accept the full ref string returned by
/// `detect_default_branch` (e.g. `refs/heads/main`) and pass it directly to
/// `git merge-base`, not prefix it with `origin/`.
#[test]
fn find_merge_base_uses_branch_ref_directly() {
    let src = diff_files_source();
    // The fixed function takes `branch_ref` and passes it directly — it must
    // NOT contain `format!("origin/{` any more (that was the old code).
    assert!(
        !src.contains(r#"format!("origin/{default_branch}")"#),
        "find_merge_base must not prepend `origin/` to the branch name — \
         detect_default_branch now returns the full ref (issue #1096)"
    );
}
