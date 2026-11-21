//! Manifest guard for `soroban_cost_lints/ui_pending/`.
//!
//! Every file in that directory must be declared in the manifest below.  If a
//! new fixture is added without updating the manifest, this test fails,
//! preventing silent bit-rot.  If a declared file disappears from disk, this
//! test also fails so that the manifest stays in sync.
//!
//! Covers issue #1090.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

/// Canonical manifest of every file that may exist under `ui_pending/`.
///
/// Add an entry here when you place a new fixture in that directory.  Remove
/// the entry if the fixture is deleted.  Entries are relative to `ui_pending/`
/// and must be sorted to keep diffs readable.
const MANIFEST: &[&str] = &[
    "README.md",
    "std_collection_in_contract.rs",
    "std_collection_in_contract.stderr",
    "storage_read_never_written.rs",
    "storage_read_never_written.stderr",
    "temporary_storage_for_persistent_data.rs",
    "temporary_storage_for_persistent_data.stderr",
    "u128_where_u64_suffices.rs",
    "u128_where_u64_suffices.stderr",
    "vec_index_in_loop.rs",
    "vec_index_in_loop.stderr",
];

/// Collect every file (not directory) directly under `ui_pending/`, returning
/// their names sorted for deterministic comparison.
fn actual_files() -> BTreeSet<String> {
    let dir = workspace_root().join("soroban_cost_lints/ui_pending");
    let entries =
        fs::read_dir(&dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));

    entries
        .filter_map(|entry| {
            let entry = entry.expect("readable directory entry");
            if entry.path().is_file() {
                Some(entry.file_name().to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn ui_pending_no_orphaned_files() {
    let declared: BTreeSet<&str> = MANIFEST.iter().copied().collect();
    let on_disk = actual_files();

    let orphaned: Vec<&str> = on_disk
        .iter()
        .filter(|name| !declared.contains(name.as_str()))
        .map(String::as_str)
        .collect();

    assert!(
        orphaned.is_empty(),
        "Files found in soroban_cost_lints/ui_pending/ that are not in the manifest; \
         add them to MANIFEST in cargo-cost-lint/tests/ui_pending_manifest.rs:\n  {orphaned:?}"
    );
}

#[test]
fn ui_pending_no_missing_files() {
    let on_disk = actual_files();
    let declared: BTreeSet<&str> = MANIFEST.iter().copied().collect();

    let missing: Vec<&&str> = declared
        .iter()
        .filter(|name| !on_disk.contains(**name))
        .collect();

    assert!(
        missing.is_empty(),
        "Files declared in the manifest are missing from soroban_cost_lints/ui_pending/; \
         remove them from MANIFEST or restore the files:\n  {missing:?}"
    );
}
