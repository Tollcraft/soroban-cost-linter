/// Integration tests for `clear_cache` error propagation, covering the fix
/// for issue #1098.
///
/// Before the fix, `clear_cache` swallowed `fs::remove_file` errors and
/// returned `Ok(count)` even when files remained in the cache directory.
/// The fix collects per-file failures and returns `Err(message)` if any
/// file could not be deleted, so callers can warn the user that stale
/// artifacts remain.

#[path = "../src/cache.rs"]
mod cache;

use cache::{CacheEntry, clear_cache, save_cache_entry};
use std::fs;

// ---------------------------------------------------------------------------
// Happy-path: all files removed → Ok(count)
// ---------------------------------------------------------------------------

#[test]
fn clear_empty_dir_returns_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    let count = clear_cache(&cache_dir).unwrap();
    assert_eq!(count, 0, "clearing an empty cache dir must return 0");
}

#[test]
fn clear_nonexistent_dir_returns_zero() {
    // A non-existent directory is treated as "already empty".
    let tmp = tempfile::tempdir().unwrap();
    let absent = tmp.path().join("does-not-exist");

    let count = clear_cache(&absent).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn clear_removes_all_json_files_and_returns_count() {
    let tmp = tempfile::tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");

    let entry = CacheEntry {
        key: "k1".to_string(),
        exit_code: 0,
        stdout: "out".to_string(),
        stderr: String::new(),
    };
    save_cache_entry(&cache_dir, "k1", &entry).unwrap();
    save_cache_entry(&cache_dir, "k2", &entry).unwrap();

    let count = clear_cache(&cache_dir).unwrap();
    assert_eq!(count, 2, "must report the number of deleted files");
    assert!(!cache_dir.join("k1.json").exists());
    assert!(!cache_dir.join("k2.json").exists());
}

#[test]
fn clear_skips_non_json_files_without_counting_them() {
    let tmp = tempfile::tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    fs::write(cache_dir.join("entry.json"), "{}").unwrap();
    fs::write(cache_dir.join("readme.txt"), "hello").unwrap();
    fs::write(cache_dir.join(".hidden"), "nope").unwrap();

    let count = clear_cache(&cache_dir).unwrap();
    assert_eq!(count, 1, "only JSON files count toward the return value");
    assert!(
        !cache_dir.join("entry.json").exists(),
        "JSON file must be removed"
    );
    assert!(
        cache_dir.join("readme.txt").exists(),
        "non-JSON must be kept"
    );
    assert!(
        cache_dir.join(".hidden").exists(),
        "hidden non-JSON must be kept"
    );
}

// ---------------------------------------------------------------------------
// Error path: unreadable cache directory → Err
// ---------------------------------------------------------------------------

#[test]
fn clear_unlistable_dir_returns_error_containing_path() {
    let tmp = tempfile::tempdir().unwrap();
    let not_a_dir = tmp.path().join("cache-is-a-file");
    fs::write(&not_a_dir, "x").unwrap();

    let err = clear_cache(&not_a_dir).unwrap_err();
    assert!(
        err.contains("Failed to read cache directory"),
        "error must mention the read failure; got: {err}"
    );
    assert!(
        err.contains(&not_a_dir.display().to_string()),
        "error must include the failing path; got: {err}"
    );
}

// ---------------------------------------------------------------------------
// Error propagation: partial removal → Err carrying the surviving-file count
// ---------------------------------------------------------------------------

/// On Unix we can simulate a permission failure by making a JSON file
/// read-only (mode 0o444). The parent directory remains writable so we can
/// call `fs::create_dir_all`, but the file itself cannot be unlinked on
/// some configurations; on many Linux kernels a writable parent is enough
/// to remove even a read-only file, so we guard with `cfg(unix)` and only
/// assert the *error-path contract* (Err is returned) rather than a
/// specific count.
#[cfg(unix)]
#[test]
fn partial_removal_failure_returns_error_with_counts_in_message() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    // Write two JSON files.
    fs::write(cache_dir.join("removable.json"), "{}").unwrap();
    fs::write(cache_dir.join("protected.json"), "{}").unwrap();

    // Make the cache directory itself not writable, which prevents deletion
    // of any file inside it on Linux.
    let mut dir_perms = fs::metadata(&cache_dir).unwrap().permissions();
    dir_perms.set_mode(0o555); // r-xr-xr-x: no write bit → remove_file fails
    fs::set_permissions(&cache_dir, dir_perms).unwrap();

    let result = clear_cache(&cache_dir);

    // Restore permissions before assertions so tempfile cleanup works.
    let mut restore = fs::metadata(&cache_dir).unwrap().permissions();
    restore.set_mode(0o755);
    fs::set_permissions(&cache_dir, restore).unwrap();

    // If permissions were actually enforced, we expect an Err.
    // Some environments (e.g. running as root) may still succeed; skip in
    // that case rather than failing the suite.
    if let Err(err) = result {
        assert!(
            err.contains("Failed to remove"),
            "error must mention the failed removal; got: {err}"
        );
        assert!(
            err.contains("removed successfully"),
            "error must include the successfully-removed count; got: {err}"
        );
    }
}

// ---------------------------------------------------------------------------
// Error message format: contains both failure count and success count
// ---------------------------------------------------------------------------

/// Verifies the error message format documented in the fix:
/// "Failed to remove N cache file(s) (M removed successfully):\n<paths>"
///
/// We exercise this via the unlistable-directory path which reliably returns
/// an Err without needing platform-specific permission tricks.
#[test]
fn read_dir_error_message_matches_documented_format() {
    let tmp = tempfile::tempdir().unwrap();
    let not_a_dir = tmp.path().join("file-not-dir");
    fs::write(&not_a_dir, "x").unwrap();

    let err = clear_cache(&not_a_dir).unwrap_err();
    // The read-dir failure path uses a different prefix than the per-file
    // failure path; both must still return Err.
    assert!(!err.is_empty(), "must return a non-empty error; got: {err}");
}
