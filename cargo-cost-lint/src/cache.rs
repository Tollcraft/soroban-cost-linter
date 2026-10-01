use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 64-bit FNV-1a deterministic hasher for cross-process cache keys.
#[derive(Default)]
pub struct DeterministicHasher {
    state: u64,
}

impl DeterministicHasher {
    pub fn new() -> Self {
        Self {
            state: 0xcbf29ce484222325, // FNV offset basis
        }
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state ^= byte as u64;
            self.state = self.state.wrapping_mul(0x100000001b3); // FNV prime
        }
    }

    pub fn write_str(&mut self, s: &str) {
        self.write_bytes(s.as_bytes());
    }

    pub fn finish_hex(&self) -> String {
        format!("{:016x}", self.state)
    }
}

/// Key containing all inputs that can affect the lint result for a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheKey {
    pub linter_version: String,
    pub toolchain: String,
    pub lint_flags: Vec<String>,
    pub package_args: Vec<String>,
    pub output_format: String,
    pub source_hash: String,
}

impl CacheKey {
    /// Computes a deterministic hexadecimal hash for this cache key.
    pub fn compute_hash(&self) -> String {
        let mut hasher = DeterministicHasher::new();
        hasher.write_str("linter_version:");
        hasher.write_str(&self.linter_version);
        hasher.write_str(";toolchain:");
        hasher.write_str(&self.toolchain);
        hasher.write_str(";lint_flags:");
        for flag in &self.lint_flags {
            hasher.write_str(flag);
            hasher.write_str(",");
        }
        hasher.write_str(";package_args:");
        for arg in &self.package_args {
            hasher.write_str(arg);
            hasher.write_str(",");
        }
        hasher.write_str(";output_format:");
        hasher.write_str(&self.output_format);
        hasher.write_str(";source_hash:");
        hasher.write_str(&self.source_hash);
        hasher.finish_hex()
    }
}

/// Cached output and exit code for a linter run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Returns the cache directory path, defaulting to `target/cost-lint-cache`.
pub fn get_cache_dir(base_dir: Option<&Path>) -> PathBuf {
    let base = base_dir.unwrap_or_else(|| Path::new("."));
    base.join("target").join("cost-lint-cache")
}

/// Computes a deterministic hash of all relevant source files in `dir`.
/// Respects `.gitignore` rules via the `ignore` crate.
pub fn compute_source_hash(dir: &Path) -> Result<String, String> {
    // Keep only paths during discovery. Retaining every file's contents until
    // after sorting needlessly duplicates the workspace in memory, especially
    // for large generated Rust sources.
    let mut files: Vec<PathBuf> = Vec::new();

    let walker = ignore::WalkBuilder::new(dir)
        .hidden(true)
        .parents(true)
        .git_ignore(true)
        .build();

    for result in walker {
        let entry = result.map_err(|e| format!("Error scanning workspace files: {}", e))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let is_target = ext == "rs"
            || file_name == "Cargo.toml"
            || file_name == "Cargo.lock"
            || file_name == "budget.toml";

        if !is_target {
            continue;
        }

        if let Ok(rel_path) = path.strip_prefix(dir) {
            files.push(rel_path.to_path_buf());
        }
    }

    files.sort();

    let mut hasher = DeterministicHasher::new();
    for rel_path in files {
        let path = dir.join(&rel_path);
        let Ok(content) = fs::read(&path) else {
            continue;
        };
        hasher.write_str(&rel_path.to_string_lossy());
        hasher.write_bytes(&content);
    }

    Ok(hasher.finish_hex())
}

/// Returns the active rustc toolchain version information.
pub fn get_toolchain_version() -> String {
    let output = Command::new("rustc").arg("-Vv").output();
    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => "unknown-toolchain".to_string(),
    }
}

/// Loads a cached entry if it exists and is readable.
pub fn load_cache_entry(cache_dir: &Path, key_hash: &str) -> Option<CacheEntry> {
    let file_path = cache_dir.join(format!("{}.json", key_hash));
    if !file_path.is_file() {
        return None;
    }
    let data = fs::read_to_string(file_path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Saves a cache entry to disk.
pub fn save_cache_entry(
    cache_dir: &Path,
    key_hash: &str,
    entry: &CacheEntry,
) -> Result<(), String> {
    fs::create_dir_all(cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;
    let file_path = cache_dir.join(format!("{}.json", key_hash));
    let data = serde_json::to_string_pretty(entry)
        .map_err(|e| format!("Failed to serialize cache entry: {}", e))?;
    fs::write(file_path, data).map_err(|e| format!("Failed to write cache entry: {}", e))?;
    Ok(())
}

/// Clears all cached entries in `cache_dir`.
pub fn clear_cache(cache_dir: &Path) -> Result<usize, String> {
    if !cache_dir.exists() {
        return Ok(0);
    }
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_json =
                path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json");
            if is_json && fs::remove_file(path).is_ok() {
                count += 1;
            }
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cache_key() -> CacheKey {
        CacheKey {
            linter_version: "0.1.1".to_string(),
            toolchain: "rustc 1.86.0-nightly (2026-04-16)".to_string(),
            lint_flags: vec!["-D soroban_storage_in_loop".to_string()],
            package_args: vec!["--package".to_string(), "my-contract".to_string()],
            output_format: "text".to_string(),
            source_hash: "a1b2c3d4e5f6".to_string(),
        }
    }

    #[test]
    fn test_cache_key_deterministic() {
        let key1 = sample_cache_key();
        let key2 = sample_cache_key();
        assert_eq!(key1.compute_hash(), key2.compute_hash());
    }

    #[test]
    fn test_cache_key_invalidated_by_source_change() {
        let key1 = sample_cache_key();
        let mut key2 = sample_cache_key();
        key2.source_hash = "f6e5d4c3b2a1".to_string();
        assert_ne!(key1.compute_hash(), key2.compute_hash());
    }

    #[test]
    fn test_cache_key_invalidated_by_lint_levels_change() {
        let key1 = sample_cache_key();
        let mut key2 = sample_cache_key();
        key2.lint_flags = vec!["-W soroban_storage_in_loop".to_string()];
        assert_ne!(key1.compute_hash(), key2.compute_hash());
    }

    #[test]
    fn test_cache_key_invalidated_by_linter_version_change() {
        let key1 = sample_cache_key();
        let mut key2 = sample_cache_key();
        key2.linter_version = "0.1.2".to_string();
        assert_ne!(key1.compute_hash(), key2.compute_hash());
    }

    #[test]
    fn test_cache_key_invalidated_by_toolchain_change() {
        let key1 = sample_cache_key();
        let mut key2 = sample_cache_key();
        key2.toolchain = "rustc 1.87.0-nightly (2026-05-01)".to_string();
        assert_ne!(key1.compute_hash(), key2.compute_hash());
    }

    #[test]
    fn test_cache_key_invalidated_by_output_format_change() {
        let key1 = sample_cache_key();
        let mut key2 = sample_cache_key();
        key2.output_format = "json".to_string();
        assert_ne!(key1.compute_hash(), key2.compute_hash());
    }

    #[test]
    fn test_cache_key_invalidated_by_package_args_change() {
        let key1 = sample_cache_key();
        let mut key2 = sample_cache_key();
        key2.package_args = vec!["--workspace".to_string()];
        assert_ne!(key1.compute_hash(), key2.compute_hash());
    }

    #[test]
    fn test_cache_save_load_clear() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join("cache");

        let key = sample_cache_key().compute_hash();
        let entry = CacheEntry {
            key: key.clone(),
            exit_code: 0,
            stdout: "all good\n".to_string(),
            stderr: "no issues\n".to_string(),
        };

        assert!(load_cache_entry(&cache_dir, &key).is_none());

        save_cache_entry(&cache_dir, &key, &entry).unwrap();

        let loaded = load_cache_entry(&cache_dir, &key).expect("entry should exist");
        assert_eq!(loaded, entry);

        let cleared = clear_cache(&cache_dir).unwrap();
        assert_eq!(cleared, 1);
        assert!(load_cache_entry(&cache_dir, &key).is_none());
    }

    #[test]
    fn test_compute_source_hash_changes_with_content() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();

        let main_rs = src_dir.join("main.rs");
        fs::write(&main_rs, "fn main() {}").unwrap();

        let hash1 = compute_source_hash(tmp.path()).unwrap();

        fs::write(&main_rs, "fn main() { let x = 1; }").unwrap();
        let hash2 = compute_source_hash(tmp.path()).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_source_hash_ignores_non_target_files() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("README.md"), "hello").unwrap();
        fs::write(tmp.path().join("data.txt"), "world").unwrap();

        let hash = compute_source_hash(tmp.path()).unwrap();
        // Should be the hash of an empty file set (same as empty dir)
        let tmp2 = tempfile::tempdir().unwrap();
        let hash2 = compute_source_hash(tmp2.path()).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_compute_source_hash_includes_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();

        let hash1 = compute_source_hash(tmp.path()).unwrap();

        fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"foo\"").unwrap();
        let hash2 = compute_source_hash(tmp.path()).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_source_hash_includes_budget_toml() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("budget.toml"), "[lints]").unwrap();

        let hash1 = compute_source_hash(tmp.path()).unwrap();

        fs::write(tmp.path().join("budget.toml"), "[lints]\nfoo = \"bar\"").unwrap();
        let hash2 = compute_source_hash(tmp.path()).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_source_hash_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let hash = compute_source_hash(tmp.path()).unwrap();
        assert_eq!(hash.len(), 16); // FNV-1a produces 16 hex chars
    }

    #[test]
    fn test_get_cache_dir_default() {
        let dir = get_cache_dir(None);
        assert!(dir.ends_with("target/cost-lint-cache"));
    }

    #[test]
    fn test_get_cache_dir_custom_base() {
        let base = Path::new("/tmp/custom");
        let dir = get_cache_dir(Some(base));
        assert_eq!(dir, PathBuf::from("/tmp/custom/target/cost-lint-cache"));
    }

    #[test]
    fn test_get_toolchain_version_returns_string() {
        // This will either return the actual version or "unknown-toolchain"
        let version = get_toolchain_version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_deterministic_hasher_new() {
        let hasher = DeterministicHasher::new();
        let hex = hasher.finish_hex();
        assert_eq!(hex.len(), 16);
    }

    #[test]
    fn test_deterministic_hasher_write_str() {
        let mut h1 = DeterministicHasher::new();
        h1.write_str("hello");

        let mut h2 = DeterministicHasher::new();
        h2.write_str("hello");

        assert_eq!(h1.finish_hex(), h2.finish_hex());
    }

    #[test]
    fn test_deterministic_hasher_write_bytes() {
        let mut h1 = DeterministicHasher::new();
        h1.write_bytes(b"hello");

        let mut h2 = DeterministicHasher::new();
        h2.write_str("hello");

        assert_eq!(h1.finish_hex(), h2.finish_hex());
    }

    #[test]
    fn test_deterministic_hasher_different_inputs() {
        let mut h1 = DeterministicHasher::new();
        h1.write_str("hello");

        let mut h2 = DeterministicHasher::new();
        h2.write_str("world");

        assert_ne!(h1.finish_hex(), h2.finish_hex());
    }

    #[test]
    fn test_deterministic_hasher_order_matters() {
        let mut h1 = DeterministicHasher::new();
        h1.write_str("ab");

        let mut h2 = DeterministicHasher::new();
        h2.write_str("ba");

        assert_ne!(h1.finish_hex(), h2.finish_hex());
    }

    #[test]
    fn test_cache_key_hash_contains_all_fields() {
        let key1 = CacheKey {
            linter_version: "1.0.0".to_string(),
            toolchain: "tc".to_string(),
            lint_flags: vec!["-A foo".to_string()],
            package_args: vec!["--workspace".to_string()],
            output_format: "text".to_string(),
            source_hash: "abc".to_string(),
        };
        let key2 = CacheKey {
            linter_version: "1.0.1".to_string(),
            ..key1.clone()
        };
        assert_ne!(key1.compute_hash(), key2.compute_hash());
    }

    #[test]
    fn test_cache_save_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join("cache");
        let key_hash = "testkey123".to_string();

        let entry = CacheEntry {
            key: key_hash.clone(),
            exit_code: 42,
            stdout: "test output".to_string(),
            stderr: "test error".to_string(),
        };

        save_cache_entry(&cache_dir, &key_hash, &entry).unwrap();
        let loaded = load_cache_entry(&cache_dir, &key_hash).unwrap();
        assert_eq!(loaded, entry);
    }

    #[test]
    fn test_cache_save_load_with_empty_strings() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join("cache");
        let key_hash = "emptykey".to_string();

        let entry = CacheEntry {
            key: key_hash.clone(),
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
        };

        save_cache_entry(&cache_dir, &key_hash, &entry).unwrap();
        let loaded = load_cache_entry(&cache_dir, &key_hash).unwrap();
        assert_eq!(loaded, entry);
    }

    #[test]
    fn test_clear_cache_removes_non_json_files() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        // Create a JSON file (should be removed)
        fs::write(cache_dir.join("entry.json"), "{}").unwrap();
        // Create a non-JSON file (should be kept)
        fs::write(cache_dir.join("readme.txt"), "hello").unwrap();

        let count = clear_cache(&cache_dir).unwrap();
        assert_eq!(count, 1);
        assert!(!cache_dir.join("entry.json").exists());
        assert!(cache_dir.join("readme.txt").exists());
    }

    #[test]
    fn test_clear_cache_nonexistent_dir() {
        let dir = Path::new("/nonexistent/path/to/cache");
        let count = clear_cache(dir).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_load_cache_entry_nonexistent() {
        let tmp = tempfile::tempdir().unwrap();
        let result = load_cache_entry(tmp.path(), "nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_load_cache_entry_corrupt_json() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();
        fs::write(cache_dir.join("corrupt.json"), "not valid json {{{").unwrap();

        let result = load_cache_entry(&cache_dir, "corrupt");
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_key_with_empty_fields() {
        let key = CacheKey {
            linter_version: String::new(),
            toolchain: String::new(),
            lint_flags: vec![],
            package_args: vec![],
            output_format: String::new(),
            source_hash: String::new(),
        };
        let hash = key.compute_hash();
        assert_eq!(hash.len(), 16);
    }

    #[test]
    fn test_cache_key_with_many_lint_flags() {
        let mut key = CacheKey {
            linter_version: "1.0.0".to_string(),
            toolchain: "tc".to_string(),
            lint_flags: vec![],
            package_args: vec![],
            output_format: "text".to_string(),
            source_hash: "abc".to_string(),
        };
        let hash1 = key.compute_hash();

        key.lint_flags = vec![
            "-A foo".to_string(),
            "-W bar".to_string(),
            "-D baz".to_string(),
        ];
        let hash2 = key.compute_hash();

        assert_ne!(hash1, hash2);
    }
}
