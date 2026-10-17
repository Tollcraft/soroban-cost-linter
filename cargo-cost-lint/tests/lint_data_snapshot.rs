//! Keeps `lint-data/` in sync with the workspace sources it snapshots.
//!
//! `build.rs` reads `lint-data/` only when the crate is built outside the
//! workspace (e.g. from a `cargo package` tarball), so drift would otherwise go
//! unnoticed until a packaged build shipped stale lint metadata.
//!
//! Run `make sync-lint-data` (or this test with `UPDATE_LINT_DATA=1`) to
//! refresh the snapshot.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Maps each snapshot path (relative to `lint-data/`) to its workspace source.
fn expected_files(workspace: &Path) -> BTreeMap<PathBuf, PathBuf> {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("lib.rs"),
        workspace.join("soroban_cost_lints/src/lib.rs"),
    );
    files.insert(
        PathBuf::from("rust-toolchain"),
        workspace.join("rust-toolchain"),
    );
    for doc in md_files(&workspace.join("docs/lints")) {
        files.insert(Path::new("docs").join(doc.file_name().unwrap()), doc);
    }
    files
}

/// Lint pages in `dir`, excluding the generated `README.md` index.
fn md_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()) == Some("md")
                && p.file_name().and_then(|n| n.to_str()) != Some("README.md")
        })
        .collect();
    files.sort();
    files
}

#[test]
fn lint_data_snapshot_matches_workspace() {
    let manifest = manifest_dir();
    let workspace = manifest.join("..");
    if !workspace.join("soroban_cost_lints/src/lib.rs").exists() {
        // Built from a packaged crate: the snapshot is the only source.
        return;
    }

    let snapshot = manifest.join("lint-data");
    let expected = expected_files(&workspace);
    let update = std::env::var_os("UPDATE_LINT_DATA").is_some();

    if update {
        update_snapshot(&workspace, &snapshot);
        return;
    }

    let mut problems = Vec::new();
    for (rel, src) in &expected {
        let want = fs::read(src).unwrap_or_else(|e| panic!("read {}: {e}", src.display()));
        match fs::read(snapshot.join(rel)) {
            Ok(got) if got == want => {}
            Ok(_) => problems.push(format!("out of date: lint-data/{}", rel.display())),
            Err(_) => problems.push(format!("missing: lint-data/{}", rel.display())),
        }
    }
    for doc in md_files(&snapshot.join("docs")) {
        let rel = Path::new("docs").join(doc.file_name().unwrap());
        if !expected.contains_key(&rel) {
            problems.push(format!("stale: lint-data/{}", rel.display()));
        }
    }

    assert!(
        problems.is_empty(),
        "cargo-cost-lint/lint-data/ is out of sync with the workspace:\n  {}\n\
         Run `make sync-lint-data` from the workspace root to refresh it.",
        problems.join("\n  ")
    );
}

/// Updates `snapshot` from `workspace`, ensuring `docs/lints` is present, readable,
/// and yields at least as many lint pages as the snapshot currently holds.
fn update_snapshot(workspace: &Path, snapshot: &Path) {
    let docs_dir = workspace.join("docs/lints");
    let entries = fs::read_dir(&docs_dir).unwrap_or_else(|e| {
        panic!(
            "failed to read lint docs directory {}: {e}",
            docs_dir.display()
        )
    });
    let mut source_docs: Vec<PathBuf> = entries
        .map(|e| {
            e.unwrap_or_else(|err| {
                panic!("failed to read entry in {}: {err}", docs_dir.display())
            })
            .path()
        })
        .filter(|p| {
            p.extension().and_then(|ext| ext.to_str()) == Some("md")
                && p.file_name().and_then(|n| n.to_str()) != Some("README.md")
        })
        .collect();
    source_docs.sort();

    let existing_docs_count = md_files(&snapshot.join("docs")).len();

    if source_docs.is_empty() {
        panic!(
            "docs directory {} yielded no lint pages; an empty docs/lints is not a valid snapshot source",
            docs_dir.display()
        );
    }
    if source_docs.len() < existing_docs_count {
        panic!(
            "docs directory {} yielded {} lint pages, fewer than the {} currently in snapshot {}; refusing to update",
            docs_dir.display(),
            source_docs.len(),
            existing_docs_count,
            snapshot.display()
        );
    }

    let expected = expected_files(workspace);
    let _ = fs::remove_dir_all(snapshot);
    for (rel, src) in &expected {
        let dest = snapshot.join(rel);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::copy(src, &dest).unwrap_or_else(|e| panic!("copy {}: {e}", src.display()));
    }
}

#[test]
fn update_snapshot_preserves_docs_when_source_missing() {
    let unique = format!(
        "scratch_lint_data_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let temp = std::env::temp_dir().join(unique);
    let scratch_workspace = temp.join("workspace");
    let scratch_snapshot = temp.join("lint-data");

    // Set up scratch workspace without docs/lints
    fs::create_dir_all(scratch_workspace.join("soroban_cost_lints/src")).unwrap();
    fs::write(
        scratch_workspace.join("soroban_cost_lints/src/lib.rs"),
        "// dummy lib.rs",
    )
    .unwrap();
    fs::write(scratch_workspace.join("rust-toolchain"), "nightly").unwrap();

    // Set up snapshot with existing docs
    let snapshot_docs = scratch_snapshot.join("docs");
    fs::create_dir_all(&snapshot_docs).unwrap();
    fs::write(snapshot_docs.join("sample_lint.md"), "# Sample Lint").unwrap();
    fs::write(scratch_snapshot.join("lib.rs"), "// dummy lib.rs").unwrap();
    fs::write(scratch_snapshot.join("rust-toolchain"), "nightly").unwrap();

    let missing_docs_dir = scratch_workspace.join("docs/lints");

    // Run the update path and assert failure
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        update_snapshot(&scratch_workspace, &scratch_snapshot);
    }));

    assert!(result.is_err(), "update must fail when docs/lints is absent");
    let payload = result.unwrap_err();
    let msg = if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        String::new()
    };

    assert!(
        msg.contains(&missing_docs_dir.display().to_string()) || msg.contains("docs/lints"),
        "failure message must name the missing directory, got: {msg}"
    );

    // Assert that the snapshot's docs/ tree survives
    assert!(
        snapshot_docs.exists(),
        "snapshot docs directory must survive failed update"
    );
    assert!(
        snapshot_docs.join("sample_lint.md").exists(),
        "snapshot lint doc must survive failed update"
    );

    // Clean up
    let _ = fs::remove_dir_all(&temp);
}
