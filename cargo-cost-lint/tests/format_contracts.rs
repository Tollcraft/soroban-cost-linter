//! Contract tests for the two machine-readable output formats that were
//! previously only checked for *acceptance* (`documented_formats_are_accepted`)
//! and never for *content*: `--format sarif` and `--format github`.
//!
//! Today `integration.rs::test_json_output` pins real findings in JSON mode and
//! nothing inspects what `generate_sarif_report` (reached from
//! `cargo-cost-lint/src/main.rs`) and the GitHub workflow-command emitter
//! actually emit. A regression in the SARIF rule ids, the deny→error /
//! warn→warning level mapping, the `region.startLine` fill-in, or the
//! `::error file=…,line=…` annotation grammar would pass CI. These tests pin
//! that behaviour end-to-end: they run the *real binary* against the
//! `test_fixtures/smoke_test` crate, which is authored to fire one deny-level
//! lint (`soroban_storage_in_loop`) and one warn-level lint
//! (`loop_invariant_storage_access`) in a single pass.
//!
//! Scope note (issue #857): this is a *separate test binary*; `integration.rs`
//! is #813's file and is not touched. The byte-level GitHub escapers
//! (`escape_github_message` / `escape_github_property` for `%`, `\r`, `\n`) are
//! pure functions living in the binary-only crate's private
//! `output_formatters` module, so they cannot be imported into a `tests/`
//! binary; they are already pinned by the in-crate unit tests in
//! `output_formatters.rs` (which #810 owns), and this file deliberately does
//! not duplicate or relocate them. Here we assert the escaping *output shape*
//! that is observable through the CLI — the `::`-delimited workflow-command
//! grammar with `file=` / `line=` properties.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Relative path of the deny-by-default lint fixture crate from the workspace
/// root. Its `budget.toml` pins `soroban_storage_in_loop = "deny"`; the
/// `&3u32` loop-invariant write additionally fires `loop_invariant_storage_access`
/// at its warn default, so one run yields the deny + warn pair the contract
/// requires.
const FIXTURE: &str = "test_fixtures/smoke_test";

/// The lint that `smoke_test` pins to `deny` (→ SARIF/GitHub error level).
const DENY_LINT: &str = "soroban_storage_in_loop";
/// The lint that `smoke_test` fires at its warn default (→ warning level).
const WARN_LINT: &str = "loop_invariant_storage_access";

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // …/cargo-cost-lint → workspace root
    p
}

/// Build the `soroban_cost_lints` cdylib from its own directory (so it honours
/// the crate's `.cargo/config.toml`) and return the profile directory that
/// `DYLINT_LIBRARY_PATH` must point at, mirroring
/// `integration.rs::test_json_output`. `cargo build` is idempotent, so calling
/// this from every test is cheap after the first.
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

/// Run the real `cargo-cost-lint` binary against the fixture crate in
/// `--format <fmt>` mode and return its stdout. Fails loudly if the binary
/// exits with a hard error (it is expected to exit non-zero when deny findings
/// are present, so the exit status is intentionally not asserted here).
///
/// `--no-cache` is mandatory here: the wrapper's cache key is derived from the
/// *linted crate's* source hash, the toolchain and the crate version — never
/// from `cargo-cost-lint`'s own formatter code (see `main.rs` cache key). A
/// warm entry would replay previously-recorded stdout and bypass the very
/// formatter these tests are meant to exercise, so the assertions could pass
/// (or fail) on stale output rather than the code under test.
fn run_fixture(fmt: &str) -> String {
    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");
    let lib_dir = ensure_lint_library_built();
    let fixture_dir = workspace_root().join(FIXTURE);
    assert!(
        fixture_dir.exists(),
        "Fixture crate not found at {fixture_dir:?}"
    );

    let output = Command::new(bin_path)
        .arg("--format")
        .arg(fmt)
        .arg("--no-cache")
        .current_dir(&fixture_dir)
        .env("DYLINT_LIBRARY_PATH", &lib_dir)
        .output()
        .expect("Failed to execute cargo-cost-lint");

    let stdout = String::from_utf8(output.stdout)
        .unwrap_or_else(|e| panic!("`{fmt}` stdout is not valid UTF-8: {e}"));
    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "`--format {fmt}` produced empty stdout.\n--- stderr ---\n{stderr}\n\
             (the fixture must emit at least one deny and one warn finding)"
        );
    }
    stdout
}

/// Locate the source line of `marker` inside the fixture's `src/lib.rs`,
/// 1-indexed. Used to assert the emitted region/line tracks the real source
/// rather than a hard-coded number that drifts when the fixture is edited.
fn fixture_line_of(marker: &str) -> usize {
    let path = workspace_root().join(FIXTURE).join("src/lib.rs");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read fixture source {path:?}: {e}"));
    text.lines()
        .position(|l| l.contains(marker))
        .unwrap_or_else(|| panic!("marker {marker:?} not found in fixture source"))
        + 1
}

#[test]
fn sarif_report_parses_and_matches_the_documented_envelope() {
    let stdout = run_fixture("sarif");

    // The whole SARIF payload must parse as one JSON document.
    let report: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("`--format sarif` output is not valid JSON: {e}\n{stdout}"));

    assert_eq!(
        report["$schema"].as_str(),
        Some("https://schemastore.azurewebsites.net/schemas/json/sarif-2.1.0-rtm.5.json"),
        "$schema must be the SARIF 2.1.0 rtm.5 schema URL"
    );
    assert_eq!(
        report["version"].as_str(),
        Some("2.1.0"),
        "SARIF version must be 2.1.0"
    );

    let runs = report["runs"].as_array().expect("`runs` must be an array");
    assert_eq!(runs.len(), 1, "exactly one run is emitted");

    let driver = &runs[0]["tool"]["driver"];
    assert_eq!(
        driver["name"].as_str(),
        Some("cargo-cost-lint"),
        "tool.driver.name must identify this linter"
    );
    assert!(
        driver["version"].is_string(),
        "tool.driver.version must be a string (the crate version)"
    );
}

#[test]
fn sarif_emits_one_reporting_descriptor_per_emitted_lint_id() {
    let stdout = run_fixture("sarif");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("valid sarif json");
    let run = &report["runs"][0];

    let results = run["results"]
        .as_array()
        .expect("`results` must be an array");
    assert!(
        !results.is_empty(),
        "the fixture run produced no SARIF results"
    );

    // The distinct set of `ruleId`s actually referenced by results.
    let mut referenced: Vec<&str> = results
        .iter()
        .filter_map(|r| r["ruleId"].as_str())
        .collect();
    referenced.sort_unstable();
    referenced.dedup();

    // The `tool.driver.rules` descriptors, which the formatter builds as one
    // entry per distinct lint name (a BTreeSet, so already sorted).
    let rules = run["tool"]["driver"]["rules"]
        .as_array()
        .expect("`tool.driver.rules` must be an array");
    let mut declared: Vec<&str> = rules
        .iter()
        .map(|rule| {
            rule["id"]
                .as_str()
                .unwrap_or_else(|| panic!("a SARIF rule is missing its `id`: {rule}"))
        })
        .collect();
    declared.sort_unstable();
    declared.dedup();

    assert_eq!(
        declared, referenced,
        "every emitted lint id must have exactly one reportingDescriptor and vice-versa"
    );

    // Each descriptor carries the pinned short-description text.
    for rule in rules {
        let id = rule["id"].as_str().unwrap();
        assert_eq!(
            rule["shortDescription"]["text"].as_str(),
            Some(&format!("Cost lint rule: {id}")[..]),
            "rule short description must reference its id"
        );
    }

    // Both the deny and the warn lint must be represented.
    for expected in [DENY_LINT, WARN_LINT] {
        assert!(
            declared.contains(&expected),
            "expected lint `{expected}` to appear among the SARIF rules; got {declared:?}"
        );
    }
}

#[test]
fn sarif_maps_levels_so_deny_is_error_and_warn_is_warning() {
    let stdout = run_fixture("sarif");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("valid sarif json");
    let results = report["runs"][0]["results"].as_array().unwrap();

    // Every emitted level must be one of the SARIF-legal values.
    for r in results {
        let level = r["level"]
            .as_str()
            .unwrap_or_else(|| panic!("a SARIF result is missing its `level`: {r}"));
        assert!(
            matches!(level, "error" | "warning" | "note"),
            "SARIF `level` must be error|warning|note, got {level:?}"
        );
    }

    let level_of = |name: &str| -> Option<String> {
        results
            .iter()
            .find(|r| r["ruleId"].as_str() == Some(name))
            .and_then(|r| r["level"].as_str())
            .map(str::to_string)
    };

    assert_eq!(
        level_of(DENY_LINT).as_deref(),
        Some("error"),
        "the deny-pinned `{DENY_LINT}` lint must map to SARIF level `error`"
    );
    assert_eq!(
        level_of(WARN_LINT).as_deref(),
        Some("warning"),
        "the default-warn `{WARN_LINT}` lint must map to SARIF level `warning`"
    );
}

#[test]
fn sarif_region_start_line_matches_the_fixture_source() {
    let stdout = run_fixture("sarif");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("valid sarif json");
    let results = report["runs"][0]["results"].as_array().unwrap();

    // `soroban_storage_in_loop` is reported at the first storage write in the
    // `bad_storage_in_loop` helper; locate that line in source rather than
    // hard-coding a number that shifts when the fixture is edited.
    let source_line = fixture_line_of("env.storage().instance().set(&1u32, &1i32);") as u64;

    let storage_result = results
        .iter()
        .find(|r| r["ruleId"].as_str() == Some(DENY_LINT))
        .expect("deny lint present in sarif results");

    let region = &storage_result["locations"][0]["physicalLocation"]["region"];
    let start_line = region["startLine"]
        .as_u64()
        .unwrap_or_else(|| panic!("`region.startLine` missing on {storage_result}"));
    assert_eq!(
        start_line, source_line,
        "SARIF region.startLine must match the fixture source line for the deny finding"
    );

    // The artifact uri is reported relative to the run directory.
    let uri = storage_result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
        .as_str()
        .unwrap_or_default();
    assert!(
        uri.ends_with("src/lib.rs"),
        "artifact uri should be the relative fixture source path, got {uri:?}"
    );
}

#[test]
fn github_annotations_use_workflow_command_grammar_with_file_and_line() {
    let stdout = run_fixture("github");

    let annotations: Vec<&str> = stdout
        .lines()
        .map(str::trim_end)
        .filter(|l| l.starts_with("::"))
        .collect();
    assert!(
        !annotations.is_empty(),
        "no GitHub workflow-command annotations were emitted:\n{stdout}"
    );

    let has_error = annotations.iter().any(|l| l.starts_with("::error "));
    let has_warning = annotations.iter().any(|l| l.starts_with("::warning "));
    assert!(
        has_error,
        "expected at least one `::error ` annotation for the deny finding; got {annotations:?}"
    );
    assert!(
        has_warning,
        "expected at least one `::warning ` annotation for the warn finding; got {annotations:?}"
    );

    // The workflow-command grammar requires a `file=` property and a `line=`
    // property on the source-located annotations.
    for line in &annotations {
        assert!(line.contains("file="), "annotation missing `file=`: {line}");
        assert!(line.contains("line="), "annotation missing `line=`: {line}");
        // Message is the segment after the final `::`.
        let body = line.rsplit_once("::").map(|(_, rest)| rest).unwrap_or(line);
        assert!(
            !body.is_empty(),
            "annotation carries an empty message: {line}"
        );
    }

    // The deny finding's annotation must be the `::error` variant, and its
    // `line=` must agree with the same source line the SARIF region reported.
    let source_line = fixture_line_of("env.storage().instance().set(&1u32, &1i32);");
    let deny_annotation = annotations
        .iter()
        .find(|l| l.starts_with("::error "))
        .expect("a ::error annotation is present");
    let reported_line = property_value(deny_annotation, "line")
        .unwrap_or_else(|| panic!("`::error` annotation has no `line=`: {deny_annotation}"));
    assert_eq!(
        reported_line.parse::<usize>(),
        Ok(source_line),
        "GitHub `line=` for the deny finding must match the fixture source line"
    );
    let file_prop = property_value(deny_annotation, "file")
        .unwrap_or_else(|| panic!("`::error` annotation has no `file=`: {deny_annotation}"));
    assert!(
        Path::new(&file_prop).ends_with(Path::new("src/lib.rs")),
        "GitHub `file=` should be the relative fixture source path, got {file_prop:?}"
    );
}

/// Extract a `,key=value` workflow-command property value from an annotation.
/// The grammar is `::{level} file=…,line=…,col=…::{message}`; the level token
/// sits before the first property, so it is dropped first. Values here never
/// contain commas (the escaper turns `,` into `%2C`), so splitting on `,` then
/// `=` is sufficient for the properties we assert on.
fn property_value(annotation: &str, key: &str) -> Option<String> {
    // Properties live between the first `::` and the message's leading `::`.
    let (_, after_prefix) = annotation.split_once("::")?;
    let (props, _) = after_prefix.split_once("::")?;
    // Drop the leading `{level}` token (`error` / `warning`) before the props.
    let props = props.split_once(' ').map(|(_, rest)| rest).unwrap_or(props);
    for segment in props.split(',') {
        let mut kv = segment.splitn(2, '=');
        if kv.next() == Some(key) {
            return kv.next().map(str::to_string);
        }
    }
    None
}
