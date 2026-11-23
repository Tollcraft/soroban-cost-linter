//! Integration tests for GitHub Actions workflow command annotation formatting.
//!
//! Covers the fix for issue #1093: `format_github_annotation` now emits
//! `title=`, `endLine=`, and `endColumn=` parameters so that GitHub PR
//! checks display the specific lint name and highlight the full expression
//! span in file diffs.

// Minimal stubs required by output_formatters.rs when it is compiled in
// isolation via `#[path]`.
#[allow(dead_code)]
mod error {
    pub type LinterResult<T> = Result<T, LinterError>;

    #[derive(Debug)]
    pub enum LinterError {
        Io(std::io::Error),
        Other(String),
    }

    impl From<std::io::Error> for LinterError {
        fn from(e: std::io::Error) -> Self {
            LinterError::Io(e)
        }
    }

    impl std::fmt::Display for LinterError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                LinterError::Io(e) => write!(f, "{e}"),
                LinterError::Other(s) => write!(f, "{s}"),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
enum ColorChoice {
    Auto,
}

#[derive(Debug)]
#[allow(dead_code)]
struct Cli {
    config: Option<String>,
    list_lints: bool,
    explain: Option<String>,
    format: output_formatters::OutputFormat,
    quiet: bool,
    verbose: bool,
    allow: Vec<String>,
    warn: Vec<String>,
    deny: Vec<String>,
    package: Vec<String>,
    workspace: bool,
    no_cache: bool,
    clear_cache: bool,
    color: ColorChoice,
    diff_only: bool,
}

#[path = "../src/output_formatters.rs"]
mod output_formatters;

use output_formatters::{LintFinding, Span, format_github_annotation};

#[allow(clippy::too_many_arguments)]
fn finding(
    name: &str,
    level: &str,
    file: &str,
    line_start: usize,
    line_end: usize,
    col_start: usize,
    col_end: usize,
    message: &str,
) -> LintFinding {
    LintFinding {
        name: name.to_string(),
        level: level.to_string(),
        file: file.to_string(),
        span: Span {
            line_start,
            line_end,
            column_start: col_start,
            column_end: col_end,
        },
        message: message.to_string(),
        help: None,
        suggestion: None,
    }
}

// ---------------------------------------------------------------------------
// title= is always present
// ---------------------------------------------------------------------------

#[test]
fn title_param_always_included() {
    let f = finding(
        "soroban_storage_in_loop",
        "warning",
        "src/lib.rs",
        10,
        10,
        5,
        20,
        "storage in loop",
    );
    let ann = format_github_annotation(&f);
    assert!(
        ann.contains("title=soroban_storage_in_loop"),
        "annotation must contain title= with the lint name; got: {ann}"
    );
}

#[test]
fn title_is_escaped_with_property_escaping() {
    let f = finding(
        "lint:with,special",
        "warning",
        "src/lib.rs",
        1,
        1,
        1,
        5,
        "msg",
    );
    let ann = format_github_annotation(&f);
    assert!(
        ann.contains("title=lint%3Awith%2Cspecial"),
        "title must be property-escaped; got: {ann}"
    );
}

// ---------------------------------------------------------------------------
// endLine= is only included when the span covers more than one line
// ---------------------------------------------------------------------------

#[test]
fn end_line_emitted_for_multiline_span() {
    let f = finding("lint_a", "warning", "src/lib.rs", 10, 12, 5, 20, "msg");
    let ann = format_github_annotation(&f);
    assert!(
        ann.contains("endLine=12"),
        "endLine must be present for a multiline span; got: {ann}"
    );
}

#[test]
fn end_line_omitted_for_single_line_span() {
    let f = finding("lint_a", "warning", "src/lib.rs", 10, 10, 5, 20, "msg");
    let ann = format_github_annotation(&f);
    assert!(
        !ann.contains("endLine="),
        "endLine must be absent when line_end == line_start; got: {ann}"
    );
}

#[test]
fn end_line_omitted_when_span_zero() {
    let f = finding("lint_a", "warning", "src/lib.rs", 0, 0, 0, 0, "msg");
    let ann = format_github_annotation(&f);
    assert!(
        !ann.contains("endLine="),
        "endLine must be absent when line_start is zero; got: {ann}"
    );
}

// ---------------------------------------------------------------------------
// endColumn= is only included when column_end > column_start
// ---------------------------------------------------------------------------

#[test]
fn end_column_emitted_when_span_wider_than_one_char() {
    let f = finding("lint_a", "warning", "src/lib.rs", 10, 10, 5, 20, "msg");
    let ann = format_github_annotation(&f);
    assert!(
        ann.contains("endColumn=20"),
        "endColumn must be present when col_end > col_start; got: {ann}"
    );
}

#[test]
fn end_column_omitted_when_col_end_equals_col_start() {
    let f = finding("lint_a", "warning", "src/lib.rs", 10, 10, 5, 5, "msg");
    let ann = format_github_annotation(&f);
    assert!(
        !ann.contains("endColumn="),
        "endColumn must be absent when col_end == col_start; got: {ann}"
    );
}

#[test]
fn end_column_omitted_when_col_start_is_zero() {
    let f = finding("lint_a", "warning", "src/lib.rs", 10, 10, 0, 0, "msg");
    let ann = format_github_annotation(&f);
    assert!(
        !ann.contains("endColumn="),
        "endColumn must be absent when col_start is zero; got: {ann}"
    );
}

// ---------------------------------------------------------------------------
// Severity mapping
// ---------------------------------------------------------------------------

#[test]
fn error_severity_maps_to_error() {
    let f = finding("lint_a", "error", "src/lib.rs", 1, 1, 1, 5, "msg");
    assert!(format_github_annotation(&f).starts_with("::error "));
}

#[test]
fn deny_severity_maps_to_error() {
    let f = finding("lint_a", "deny", "src/lib.rs", 1, 1, 1, 5, "msg");
    assert!(format_github_annotation(&f).starts_with("::error "));
}

#[test]
fn warning_severity_maps_to_warning() {
    let f = finding("lint_a", "warning", "src/lib.rs", 1, 1, 1, 5, "msg");
    assert!(format_github_annotation(&f).starts_with("::warning "));
}

// ---------------------------------------------------------------------------
// Full annotation shape
// ---------------------------------------------------------------------------

#[test]
fn full_annotation_includes_all_params() {
    let f = finding(
        "soroban_storage_in_loop",
        "error",
        "src/lib.rs",
        10,
        12,
        5,
        20,
        "storage in loop",
    );
    let ann = format_github_annotation(&f);
    assert!(ann.contains("line=10"), "line= missing; got: {ann}");
    assert!(ann.contains("endLine=12"), "endLine= missing; got: {ann}");
    assert!(ann.contains("col=5"), "col= missing; got: {ann}");
    assert!(
        ann.contains("endColumn=20"),
        "endColumn= missing; got: {ann}"
    );
    assert!(
        ann.contains("title=soroban_storage_in_loop"),
        "title= missing; got: {ann}"
    );
    assert!(
        ann.ends_with("::storage in loop"),
        "message missing; got: {ann}"
    );
}

#[test]
fn annotation_without_file_still_has_title() {
    let f = finding("lint_a", "warning", "", 0, 0, 0, 0, "msg");
    let ann = format_github_annotation(&f);
    assert!(
        ann.contains("title=lint_a"),
        "title must appear even without a file; got: {ann}"
    );
}
