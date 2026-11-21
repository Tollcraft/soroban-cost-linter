//! Tests that `clear_cache` propagates file-removal errors (issue #1098).
//!
//! Previously `clear_cache` swallowed `remove_file` failures with `.is_ok()`,
//! silently under-reporting the number of cleared entries. After the fix,
//! the first failed deletion returns `Err` so callers can warn the user.

#[path = "../src/cache.rs"]
#[allow(dead_code)]
mod cache;

use cache::{CacheEntry, clear_cache, save_cache_entry};
use std::fs;
use tempfile::tempdir;

fn dummy_entry(key: &str) -> CacheEntry {
    CacheEntry {
        key: key.to_string(),
        exit_code: 0,
        stdout: "ok\n".to_string(),
        stderr: String::new(),
    }
}

#[test]
fn clear_cache_returns_zero_for_nonexistent_directory() {
    let tmp = tempdir().unwrap();
    let absent = tmp.path().join("does_not_exist");
    let count = clear_cache(&absent).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn clear_cache_removes_all_json_files_and_returns_count() {
    let tmp = tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");

    save_cache_entry(&cache_dir, "key1", &dummy_entry("key1")).unwrap();
    save_cache_entry(&cache_dir, "key2", &dummy_entry("key2")).unwrap();
    save_cache_entry(&cache_dir, "key3", &dummy_entry("key3")).unwrap();

    let count = clear_cache(&cache_dir).unwrap();
    assert_eq!(count, 3);

    let remaining: Vec<_> = fs::read_dir(&cache_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()).unwrap_or("") == "json")
        .collect();
    assert!(remaining.is_empty(), "all json files must be removed");
}

#[test]
fn clear_cache_ignores_non_json_files() {
    let tmp = tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    save_cache_entry(&cache_dir, "key1", &dummy_entry("key1")).unwrap();
    fs::write(cache_dir.join("readme.txt"), "metadata").unwrap();
    fs::write(cache_dir.join("lock"), "pid=42").unwrap();

    let count = clear_cache(&cache_dir).unwrap();
    assert_eq!(count, 1, "only the .json file should be counted");

    assert!(cache_dir.join("readme.txt").exists());
    assert!(cache_dir.join("lock").exists());
    assert!(!cache_dir.join("key1.json").exists());
}

#[test]
fn clear_cache_on_file_instead_of_directory_returns_error() {
    let tmp = tempdir().unwrap();
    let not_a_dir = tmp.path().join("not-a-dir");
    fs::write(&not_a_dir, "content").unwrap();

    let err = clear_cache(&not_a_dir).unwrap_err();
    assert!(
        err.contains("Failed to read cache directory"),
        "error must describe the read failure: {err}"
    );
    assert!(
        err.contains(&not_a_dir.display().to_string()),
        "error must include the path: {err}"
    );
}

#[test]
fn clear_cache_empty_directory_returns_zero() {
    let tmp = tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    let count = clear_cache(&cache_dir).unwrap();
    assert_eq!(count, 0);
}

#[cfg(unix)]
#[test]
fn clear_cache_propagates_permission_error_on_read_only_file() {
    use std::os::unix::fs::PermissionsExt;

    // Skip on root (root can bypass directory permissions).
    // SAFETY: getuid is a standard POSIX C function with no preconditions.
    unsafe extern "C" {
        fn getuid() -> u32;
    }
    if unsafe { getuid() } == 0 {
        return;
    }

    let tmp = tempdir().unwrap();
    let cache_dir = tmp.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    let json_path = cache_dir.join("locked.json");
    fs::write(&json_path, "{}").unwrap();

    // In Unix, removing a file requires write permission on the parent directory.
    fs::set_permissions(&cache_dir, fs::Permissions::from_mode(0o555)).unwrap();

    let res = clear_cache(&cache_dir);

    // Restore write permissions so tempdir cleanup succeeds.
    let _ = fs::set_permissions(&cache_dir, fs::Permissions::from_mode(0o755));

    let err = res.unwrap_err();
    assert!(
        err.contains("Failed to remove cache file"),
        "error must describe the removal failure: {err}"
    );
}
