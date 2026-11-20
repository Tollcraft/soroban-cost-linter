use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_generate-lint-docs"))
}

#[test]
fn test_empty_parse_refuses_to_overwrite_docs_without_override() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let root = tmp.path();

    let lib_dir = root.join("soroban_cost_lints/src");
    let docs_lints = root.join("docs/lints");
    fs::create_dir_all(&lib_dir).expect("failed to create lib_dir");
    fs::create_dir_all(&docs_lints).expect("failed to create docs_lints");

    // Write empty lib.rs with no declare_lint! macros
    fs::write(
        lib_dir.join("lib.rs"),
        "// empty file without any lint definitions\n",
    )
    .expect("failed to write lib.rs");

    // Pre-populate documentation files with initial sentinel text
    let readme_path = docs_lints.join("README.md");
    let catalog_path = root.join("docs/lint_catalog.md");
    fs::write(&readme_path, "PRESERVED_README_CONTENT").expect("failed to write README.md");
    fs::write(&catalog_path, "PRESERVED_CATALOG_CONTENT").expect("failed to write lint_catalog.md");

    // Invoking without --allow-empty must fail
    let output = Command::new(binary_path())
        .args(["--workspace-root", root.to_str().unwrap()])
        .output()
        .expect("failed to execute generate-lint-docs");

    assert!(
        !output.status.success(),
        "command should fail when parsing zero lints"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("soroban_cost_lints") && stderr.contains("lib.rs"),
        "stderr should emit a diagnostic pointing to soroban_cost_lints/src/lib.rs; got: {stderr}"
    );

    // Verify existing docs files were not wiped or modified
    let readme_after = fs::read_to_string(&readme_path).expect("failed to read README.md");
    assert_eq!(
        readme_after, "PRESERVED_README_CONTENT",
        "README.md must not be overwritten on empty parse"
    );

    let catalog_after = fs::read_to_string(&catalog_path).expect("failed to read lint_catalog.md");
    assert_eq!(
        catalog_after, "PRESERVED_CATALOG_CONTENT",
        "lint_catalog.md must not be overwritten on empty parse"
    );
}

#[test]
fn test_empty_parse_succeeds_with_allow_empty_override() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let root = tmp.path();

    let lib_dir = root.join("soroban_cost_lints/src");
    let docs_lints = root.join("docs/lints");
    fs::create_dir_all(&lib_dir).expect("failed to create lib_dir");
    fs::create_dir_all(&docs_lints).expect("failed to create docs_lints");

    fs::write(
        lib_dir.join("lib.rs"),
        "// empty file without any lint definitions\n",
    )
    .expect("failed to write lib.rs");

    let readme_path = docs_lints.join("README.md");
    let catalog_path = root.join("docs/lint_catalog.md");
    fs::write(&readme_path, "PRESERVED_README_CONTENT").expect("failed to write README.md");
    fs::write(&catalog_path, "PRESERVED_CATALOG_CONTENT").expect("failed to write lint_catalog.md");

    // Invoking with --allow-empty must succeed and regenerate
    let output = Command::new(binary_path())
        .args(["--allow-empty", "--workspace-root", root.to_str().unwrap()])
        .output()
        .expect("failed to execute generate-lint-docs");

    assert!(
        output.status.success(),
        "command should succeed with --allow-empty override; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let readme_after = fs::read_to_string(&readme_path).expect("failed to read README.md");
    assert_ne!(
        readme_after, "PRESERVED_README_CONTENT",
        "README.md should be regenerated when --allow-empty is passed"
    );
}

#[test]
fn test_valid_parse_succeeds_normally() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let root = tmp.path();

    let lib_dir = root.join("soroban_cost_lints/src");
    let docs_lints = root.join("docs/lints");
    fs::create_dir_all(&lib_dir).expect("failed to create lib_dir");
    fs::create_dir_all(&docs_lints).expect("failed to create docs_lints");

    let lib_rs = r#"
rustc_session::declare_lint! {
    pub TEST_LINT,
    Warn,
    "test lint description"
}

pub const LINT_METADATA: &[LintMeta] = &[
    LintMeta {
        name: "test_lint",
        category: LintCategory::Host,
        description: "test lint description",
        rationale: "test rationale",
    },
];
"#;
    fs::write(lib_dir.join("lib.rs"), lib_rs).expect("failed to write lib.rs");

    let status = Command::new(binary_path())
        .args(["--workspace-root", root.to_str().unwrap()])
        .status()
        .expect("failed to execute generate-lint-docs");

    assert!(status.success(), "command should succeed with valid lints");
    assert!(docs_lints.join("README.md").exists());
    assert!(root.join("docs/lint_catalog.md").exists());
}
