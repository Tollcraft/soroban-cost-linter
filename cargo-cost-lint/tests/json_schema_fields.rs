//! Integration test: `--format json` output conforms to the documented
//! `LintFinding` schema.
//!
//! Covers issue #1094: systematic verification that optional fields (`help`,
//! `suggestion`) are omitted from JSON when absent, all coordinate fields in
//! `span` are non-negative integers, required fields are always present, and
//! output is valid NDJSON (one JSON object per line, no array wrapper).

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn ensure_lint_library_built() -> PathBuf {
    let root = workspace_root();
    let lint_dir = root.join("soroban_cost_lints");

    let status = Command::new(env!("CARGO"))
        .arg("build")
        .current_dir(&lint_dir)
        .status()
        .expect("Failed to spawn `cargo build` for soroban_cost_lints");
    assert!(status.success(), "Failed to build soroban_cost_lints");

    match std::env::var("CARGO_TARGET_DIR") {
        Ok(dir) => PathBuf::from(dir).join("debug"),
        Err(_) => root.join("target").join("debug"),
    }
}

fn run_json() -> String {
    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");
    let lib_dir = ensure_lint_library_built();
    let fixture_dir = workspace_root().join("test_fixtures/smoke_test");

    let output = Command::new(bin_path)
        .args(["--format", "json", "--no-cache"])
        .current_dir(&fixture_dir)
        .env("DYLINT_LIBRARY_PATH", &lib_dir)
        .output()
        .expect("Failed to execute cargo-cost-lint");

    let stdout =
        String::from_utf8(output.stdout).expect("`--format json` stdout is not valid UTF-8");
    assert!(
        !stdout.trim().is_empty(),
        "`--format json` produced no findings; the fixture must emit at least one finding"
    );
    stdout
}

/// Parse the NDJSON output into a list of JSON objects.
fn parse_findings(stdout: &str) -> Vec<serde_json::Value> {
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("NDJSON line is not valid JSON: {e}\n  line: {line}"))
        })
        .collect()
}

#[test]
fn json_output_is_ndjson_not_an_array() {
    // The whole stdout must NOT parse as a single JSON array; each line must be
    // an independent JSON object.
    let stdout = run_json();
    let full_parse: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    if let Ok(v) = full_parse {
        assert!(
            !v.is_array(),
            "JSON output must be NDJSON (one object per line), \
             not a single JSON array"
        );
    }
    // Regardless of whether the whole thing parses, every non-empty line must
    // be a valid standalone JSON object.
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        let val: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line is not valid JSON: {e}\n  line: {line}"));
        assert!(val.is_object(), "each NDJSON line must be a JSON object");
    }
}

#[test]
fn json_findings_have_required_fields() {
    let stdout = run_json();
    let findings = parse_findings(&stdout);
    assert!(!findings.is_empty(), "must have at least one finding");

    for (i, f) in findings.iter().enumerate() {
        for field in &["name", "level", "file", "message"] {
            assert!(
                f.get(*field).is_some_and(|v| v.is_string()),
                "finding[{i}] must have a string field `{field}`; got: {f}"
            );
        }
        assert!(
            f.get("span").is_some_and(|v| v.is_object()),
            "finding[{i}] must have an object field `span`; got: {f}"
        );
    }
}

#[test]
fn json_span_fields_are_non_negative_integers() {
    let stdout = run_json();
    let findings = parse_findings(&stdout);

    for (i, f) in findings.iter().enumerate() {
        let span = f["span"]
            .as_object()
            .unwrap_or_else(|| panic!("finding[{i}] missing `span` object"));
        for coord in &["line_start", "line_end", "column_start", "column_end"] {
            let val = span
                .get(*coord)
                .unwrap_or_else(|| panic!("finding[{i}].span missing `{coord}`"));
            assert!(
                val.is_u64(),
                "finding[{i}].span.{coord} must be a non-negative integer; got {val}"
            );
        }
    }
}

#[test]
fn json_optional_fields_absent_when_none() {
    // `help` and `suggestion` are decorated with `#[serde(skip_serializing_if =
    // "Option::is_none")]`; they must not appear in the JSON object when the
    // lint did not populate them. In the smoke_test fixture, findings do not
    // populate `suggestion`, so verifying that `suggestion` is omitted from
    // the JSON output validates the skip_serializing_if contract.
    let stdout = run_json();
    let findings = parse_findings(&stdout);

    let without_suggestion: Vec<_> = findings
        .iter()
        .filter(|f| f.get("suggestion").is_none())
        .collect();

    assert!(
        !without_suggestion.is_empty(),
        "expected at least one finding without `suggestion` \
         to verify skip_serializing_if behaviour; got findings: {findings:#?}"
    );
}

#[test]
fn json_optional_fields_present_when_set() {
    // When `help` or `suggestion` IS populated, it must be a non-null string.
    let stdout = run_json();
    let findings = parse_findings(&stdout);

    for (i, f) in findings.iter().enumerate() {
        for opt in &["help", "suggestion"] {
            if let Some(v) = f.get(*opt) {
                assert!(
                    v.is_string(),
                    "finding[{i}].{opt} is present but not a string; got {v}"
                );
                assert!(
                    !v.as_str().unwrap_or("").is_empty(),
                    "finding[{i}].{opt} is present but empty"
                );
            }
        }
    }
}
