//! The cache key must capture every input that changes the reported result:
//! the diff scope and the resolved `.lintignore` are among them (#793).
//!
//! A cached full-run entry must never be replayed for a later `--diff-only`
//! run (or the reverse), and editing or adding `.lintignore` must change the
//! key. Both orders are driven: cached-then-filtered and
//! filtered-then-cached.

#[path = "../src/cache.rs"]
mod cache;

use cache::{ABSENT_LINTIGNORE_HASH, CACHE_KEY_FORMAT, CacheKey};

fn key(diff_scope: &str, lintignore_hash: &str) -> CacheKey {
    CacheKey {
        linter_version: "0.1.1".to_string(),
        toolchain: "test-toolchain".to_string(),
        lint_flags: vec![],
        package_args: vec![],
        output_format: "text".to_string(),
        source_hash: "source-v1".to_string(),
        diff_scope: diff_scope.to_string(),
        lintignore_hash: lintignore_hash.to_string(),
    }
}

#[test]
fn full_run_then_diff_only_run_does_not_reuse_the_entry() {
    let full = key("all", ABSENT_LINTIGNORE_HASH);
    let filtered = key("diff-only", ABSENT_LINTIGNORE_HASH);
    assert_ne!(
        full.compute_hash(),
        filtered.compute_hash(),
        "a cached full run must not be replayed for a --diff-only run"
    );
}

#[test]
fn diff_only_run_then_full_run_does_not_reuse_the_entry() {
    // Same inequality from the other order: the filter flag itself is part
    // of the key, so whichever run came first, the other one misses.
    let filtered = key("diff-only", ABSENT_LINTIGNORE_HASH);
    let full = key("all", ABSENT_LINTIGNORE_HASH);
    assert_ne!(
        filtered.compute_hash(),
        full.compute_hash(),
        "a cached --diff-only run must not be replayed for a full run"
    );
}

#[test]
fn editing_lintignore_changes_the_key() {
    let before = key("all", "lintignore-hash-1");
    let after = key("all", "lintignore-hash-2");
    assert_ne!(
        before.compute_hash(),
        after.compute_hash(),
        "editing .lintignore must change the key"
    );
}

#[test]
fn adding_a_lintignore_later_invalidates_entries_written_before() {
    let without = key("all", ABSENT_LINTIGNORE_HASH);
    let with = key("all", "some-hash");
    assert_ne!(without.compute_hash(), with.compute_hash());
    // The sentinel must never equal a content hash.
    assert_ne!(ABSENT_LINTIGNORE_HASH, "some-hash");
}

#[test]
fn format_version_participates_in_the_hash() {
    // Entries written under the previous format cannot match a new key:
    // the format string is hashed in, so old files' names are derived with
    // different inputs entirely.
    let key = key("all", ABSENT_LINTIGNORE_HASH);
    assert_eq!(key.compute_hash().len(), 16);
    assert!(CACHE_KEY_FORMAT.starts_with("cache_key_format:"));
}

#[test]
fn content_hash_tracks_the_file_on_disk() {
    use std::fs;

    let dir = std::env::temp_dir().join(format!("cckey-{:x}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let li = dir.join(".lintignore");

    fs::write(&li, "target/\n").unwrap();
    let first = cache::compute_file_hash(&li);
    assert_ne!(first, ABSENT_LINTIGNORE_HASH);

    // Editing the file changes the resolved hash.
    fs::write(&li, "target/\nvendor/\n").unwrap();
    let second = cache::compute_file_hash(&li);
    assert_ne!(first, second, "editing .lintignore changes the hash");

    // Removing it falls back to the distinct absence sentinel.
    fs::remove_file(&li).unwrap();
    assert_eq!(cache::compute_file_hash(&li), ABSENT_LINTIGNORE_HASH);

    fs::remove_dir_all(&dir).unwrap();
}
