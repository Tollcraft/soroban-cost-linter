use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir
}

#[test]
fn test_no_references_to_lint_registry_in_docs() {
    let root = workspace_root();
    let doc_files = [
        root.join("CONTRIBUTING.md"),
        root.join("DEVELOPING_LINTS.md"),
        root.join("README.md"),
        root.join("docs/lints/README.md"),
        root.join("docs/lint_catalog.md"),
    ];

    for path in doc_files {
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
        assert!(
            !content.contains("lint-registry.json"),
            "found obsolete reference to 'lint-registry.json' in {}",
            path.display()
        );
    }
}

#[test]
fn test_documentation_references_actual_generator_outputs() {
    let root = workspace_root();

    let contributing =
        fs::read_to_string(root.join("CONTRIBUTING.md")).expect("failed to read CONTRIBUTING.md");
    assert!(
        contributing.contains("docs/lints/README.md"),
        "CONTRIBUTING.md must reference 'docs/lints/README.md'"
    );
    assert!(
        contributing.contains("docs/lint_catalog.md"),
        "CONTRIBUTING.md must reference 'docs/lint_catalog.md'"
    );

    let developing_lints = fs::read_to_string(root.join("DEVELOPING_LINTS.md"))
        .expect("failed to read DEVELOPING_LINTS.md");
    assert!(
        developing_lints.contains("docs/lints/README.md"),
        "DEVELOPING_LINTS.md must reference 'docs/lints/README.md'"
    );
    assert!(
        developing_lints.contains("docs/lint_catalog.md"),
        "DEVELOPING_LINTS.md must reference 'docs/lint_catalog.md'"
    );
}
