use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_generate-lint-docs"))
}

fn create_test_workspace() -> tempfile::TempDir {
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

    let summary = "# Summary\n\n## Lints\n\n* [test_lint](lints/test_lint.md)\n";
    fs::write(root.join("docs/SUMMARY.md"), summary).expect("failed to write SUMMARY.md");
    fs::write(docs_lints.join("test_lint.md"), "# Test Lint\n")
        .expect("failed to write test_lint.md");

    tmp
}

#[test]
fn test_generated_docs_use_lf_only() {
    let tmp = create_test_workspace();
    let root = tmp.path();

    let status = Command::new(binary_path())
        .args(["--workspace-root", root.to_str().unwrap()])
        .status()
        .expect("failed to execute generate-lint-docs");
    assert!(status.success(), "generate-lint-docs failed");

    let readme = fs::read(root.join("docs/lints/README.md")).expect("failed to read README.md");
    assert!(!readme.is_empty());
    assert!(
        !readme.windows(2).any(|w| w == b"\r\n"),
        "README.md contains CRLF line endings"
    );

    let catalog =
        fs::read(root.join("docs/lint_catalog.md")).expect("failed to read lint_catalog.md");
    assert!(!catalog.is_empty());
    assert!(
        !catalog.windows(2).any(|w| w == b"\r\n"),
        "lint_catalog.md contains CRLF line endings"
    );
}

#[test]
fn test_check_mode_succeeds_with_crlf_files() {
    let tmp = create_test_workspace();
    let root = tmp.path();

    // First generate the files (LF)
    let status = Command::new(binary_path())
        .args(["--workspace-root", root.to_str().unwrap()])
        .status()
        .expect("failed to execute generate-lint-docs");
    assert!(status.success());

    // Convert generated docs to CRLF
    let readme_path = root.join("docs/lints/README.md");
    let readme_crlf = fs::read_to_string(&readme_path)
        .unwrap()
        .replace("\r\n", "\n")
        .replace('\n', "\r\n");
    fs::write(&readme_path, readme_crlf).unwrap();

    let catalog_path = root.join("docs/lint_catalog.md");
    let catalog_crlf = fs::read_to_string(&catalog_path)
        .unwrap()
        .replace("\r\n", "\n")
        .replace('\n', "\r\n");
    fs::write(&catalog_path, catalog_crlf).unwrap();

    // Verify --check succeeds even with CRLF line endings
    let check_status = Command::new(binary_path())
        .args(["--check", "--workspace-root", root.to_str().unwrap()])
        .status()
        .expect("failed to execute generate-lint-docs --check");
    assert!(
        check_status.success(),
        "--check should succeed when files differ only by CRLF line endings"
    );
}

#[test]
fn test_check_mode_succeeds_with_lf_files() {
    let tmp = create_test_workspace();
    let root = tmp.path();

    // Generate files (LF)
    let status = Command::new(binary_path())
        .args(["--workspace-root", root.to_str().unwrap()])
        .status()
        .expect("failed to execute generate-lint-docs");
    assert!(status.success());

    // Verify --check succeeds with LF line endings
    let check_status = Command::new(binary_path())
        .args(["--check", "--workspace-root", root.to_str().unwrap()])
        .status()
        .expect("failed to execute generate-lint-docs --check");
    assert!(
        check_status.success(),
        "--check should succeed with canonical LF files"
    );
}
