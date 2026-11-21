/// Integration tests for cache key invalidation on config and cargo-config
/// changes, covering the fix for issue #1097.
///
/// Before the fix, `compute_source_hash` only hashed Rust source files,
/// `Cargo.toml`, `Cargo.lock`, and `budget.toml`.  Editing `.cargo/config.toml`
/// (which controls linker flags, compiler settings, and Dylint paths) or the
/// user-supplied `--config` file did not change the hash, so stale cache
/// entries were replayed even though the lint results could differ.

#[path = "../src/cache.rs"]
mod cache;

use cache::compute_source_hash;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// .cargo/config.toml invalidates the cache hash (#1097)
// ---------------------------------------------------------------------------

#[test]
fn adding_cargo_config_toml_changes_the_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    // Baseline: no .cargo/config.toml present.
    let hash_before = compute_source_hash(dir, None).unwrap();

    // Add .cargo/config.toml.
    let cargo_dir = dir.join(".cargo");
    fs::create_dir_all(&cargo_dir).unwrap();
    fs::write(
        cargo_dir.join("config.toml"),
        "[build]\ntarget = \"wasm32-unknown-unknown\"\n",
    )
    .unwrap();

    let hash_after = compute_source_hash(dir, None).unwrap();

    assert_ne!(
        hash_before, hash_after,
        "adding .cargo/config.toml must change the source hash"
    );
}

#[test]
fn editing_cargo_config_toml_changes_the_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let cargo_dir = dir.join(".cargo");
    fs::create_dir_all(&cargo_dir).unwrap();
    let config_path = cargo_dir.join("config.toml");

    fs::write(
        &config_path,
        "[build]\ntarget = \"wasm32-unknown-unknown\"\n",
    )
    .unwrap();
    let hash_v1 = compute_source_hash(dir, None).unwrap();

    fs::write(
        &config_path,
        "[build]\ntarget = \"wasm32-unknown-unknown\"\nrustflags = [\"-C\", \"opt-level=s\"]\n",
    )
    .unwrap();
    let hash_v2 = compute_source_hash(dir, None).unwrap();

    assert_ne!(
        hash_v1, hash_v2,
        "editing .cargo/config.toml must change the source hash"
    );
}

#[test]
fn cargo_config_toml_absent_produces_stable_hash() {
    // If .cargo/config.toml is absent, the hash must still be deterministic.
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    let hash1 = compute_source_hash(tmp1.path(), None).unwrap();
    let hash2 = compute_source_hash(tmp2.path(), None).unwrap();
    assert_eq!(
        hash1, hash2,
        "two empty directories with no config must yield the same hash"
    );
}

// ---------------------------------------------------------------------------
// Custom --config file invalidates the cache hash (#1097)
// ---------------------------------------------------------------------------

#[test]
fn adding_custom_config_changes_the_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let custom_config = tmp.path().join("my-budget.toml");

    // Baseline: no custom config.
    let hash_without = compute_source_hash(dir, None).unwrap();

    fs::write(
        &custom_config,
        "[lints]\nsoroban_storage_in_loop = \"deny\"\n",
    )
    .unwrap();
    let hash_with = compute_source_hash(dir, Some(&custom_config)).unwrap();

    assert_ne!(
        hash_without, hash_with,
        "providing a custom config must change the source hash"
    );
}

#[test]
fn editing_custom_config_changes_the_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let custom_config = tmp.path().join("my-budget.toml");

    fs::write(
        &custom_config,
        "[lints]\nsoroban_storage_in_loop = \"warn\"\n",
    )
    .unwrap();
    let hash_v1 = compute_source_hash(dir, Some(&custom_config)).unwrap();

    fs::write(
        &custom_config,
        "[lints]\nsoroban_storage_in_loop = \"deny\"\n",
    )
    .unwrap();
    let hash_v2 = compute_source_hash(dir, Some(&custom_config)).unwrap();

    assert_ne!(
        hash_v1, hash_v2,
        "editing the custom config file must change the source hash"
    );
}

#[test]
fn same_custom_config_content_produces_same_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let custom_config = tmp.path().join("budget.toml");
    let content = "[lints]\nsoroban_storage_in_loop = \"deny\"\n";

    fs::write(&custom_config, content).unwrap();
    let hash1 = compute_source_hash(dir, Some(&custom_config)).unwrap();
    let hash2 = compute_source_hash(dir, Some(&custom_config)).unwrap();

    assert_eq!(
        hash1, hash2,
        "identical config content must produce the same hash on repeated calls"
    );
}

#[test]
fn both_cargo_config_and_custom_config_contribute_independently() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let cargo_dir = dir.join(".cargo");
    fs::create_dir_all(&cargo_dir).unwrap();
    let custom_config = tmp.path().join("budget.toml");

    // Write both config files.
    fs::write(
        cargo_dir.join("config.toml"),
        "[build]\ntarget = \"wasm32v1-none\"\n",
    )
    .unwrap();
    fs::write(
        &custom_config,
        "[lints]\nsoroban_storage_in_loop = \"deny\"\n",
    )
    .unwrap();

    let hash_both = compute_source_hash(dir, Some(&custom_config)).unwrap();

    // A different custom config must produce a different hash even though
    // .cargo/config.toml is the same.
    let custom_config2 = tmp.path().join("budget2.toml");
    fs::write(
        &custom_config2,
        "[lints]\nsoroban_storage_in_loop = \"allow\"\n",
    )
    .unwrap();
    let hash_different_custom = compute_source_hash(dir, Some(&custom_config2)).unwrap();

    assert_ne!(
        hash_both, hash_different_custom,
        "changing only the custom config must change the hash"
    );
}

// ---------------------------------------------------------------------------
// Rust-source changes still invalidate independently
// ---------------------------------------------------------------------------

#[test]
fn source_changes_still_invalidate_even_with_config_present() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let custom_config = tmp.path().join("budget.toml");
    fs::write(&custom_config, "[lints]\n").unwrap();

    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.rs"), "fn foo() {}").unwrap();

    let hash1 = compute_source_hash(dir, Some(&custom_config)).unwrap();

    fs::write(src.join("lib.rs"), "fn foo() { let _x = 1; }").unwrap();
    let hash2 = compute_source_hash(dir, Some(&custom_config)).unwrap();

    assert_ne!(
        hash1, hash2,
        "source changes must still invalidate the hash when a custom config is present"
    );
}

// helper to avoid adding tempfile to [dependencies]
fn _ensure_tempfile_compiles() {
    let _ = Path::new(".");
}
