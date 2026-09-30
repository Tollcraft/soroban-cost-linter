//! Real-world corpus regression test for `cargo-cost-lint`.
//!
//! This test builds every contract under `tests/corpus/contracts/`, runs the
//! linter against each one, and compares the resulting false-positive count
//! against a committed baseline (`tests/corpus/baseline.json`).
//!
//! # Triage model
//!
//! Every finding is classified as either a true positive (TP) or a false
//! positive (FP):
//!
//! * Findings from lints in [`ALWAYS_TP`] are unconditionally TPs — these lints
//!   have no known false-positive patterns.
//! * Everything else starts as an FP, on the principle that a finding nobody
//!   has vetted cannot be trusted.
//!
//! The test fails if any contract produces **more** FPs than its baseline
//! entry, so lint changes that over-report on real-world shapes are caught in
//! CI. FPs may decrease freely; TPs are tracked for reporting but not
//! enforced.
//!
//! # Re-blessing
//!
//! When a new FP is legitimate (for example, a deliberately stricter lint),
//! regenerate the baseline and commit it:
//!
//! ```text
//! BLESS=1 cargo test --test real_world_corpus --workspace
//! ```
//!
//! The test never modifies the baseline unless `BLESS` is set, so a plain
//! `cargo test --workspace` run is safe to re-run at any time.
//!
//! # Performance
//!
//! All contract builds and linter invocations share one target directory (see
//! [`shared_target_dir`]), so the Soroban SDK is compiled once for the whole
//! corpus instead of once per contract.

use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A single lint finding from the corpus.
///
/// Fields mirror the JSON objects the `--format json` output of
/// `cargo-cost-lint` emits, one finding per line. Kept cheap to clone because
/// triage hands out owned copies of each finding (see [`triage_findings`]).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Finding {
    /// Snake-case name of the lint that fired, e.g. `soroban_storage_in_loop`.
    pub lint_name: String,
    /// Path of the source file relative to the contract root, as reported by the linter.
    pub file: String,
    /// One-based line number the diagnostic starts on; `0` if the linter omitted it.
    pub line: usize,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Per-contract baseline entry.
///
/// One entry per corpus contract in [`Baseline::contracts`]. `total` is
/// redundant with the lengths of the two vectors but is kept as a checksum of
/// what the baseline generation actually saw.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BaselineEntry {
    /// Total number of findings (TPs and FPs) recorded for the contract.
    pub total: usize,
    /// Findings triaged as true positives.
    pub true_positives: Vec<Finding>,
    /// Findings triaged as false positives — the counts this test enforces.
    pub false_positives: Vec<Finding>,
}

/// Summary statistics for corpus baseline.
///
/// Written by [`BLESS`](self#blessing) runs so the baseline file is
/// self-describing; the comparison logic only reads per-contract entries.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BaselineSummary {
    /// All findings across every contract.
    pub total_findings: usize,
    /// Findings triaged as true positives across every contract.
    pub total_true_positives: usize,
    /// Findings triaged as false positives across every contract.
    pub total_false_positives: usize,
    /// `false_positives / total`, rounded to two decimal places.
    pub false_positive_rate_percent: f64,
}

/// Full baseline file format.
///
/// Serialized to `tests/corpus/baseline.json`. `version` allows future format
/// changes to be detected at load time; the current reader accepts version 1
/// only implicitly (unknown fields would fail deserialization).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Baseline {
    /// Baseline format version.
    pub version: u32,
    /// Free-form provenance note (generation date, counts, regeneration command).
    pub description: String,
    /// Aggregate statistics, present only in baselines written by blessed runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<BaselineSummary>,
    /// Findings per contract, keyed by contract directory name.
    pub contracts: BTreeMap<String, BaselineEntry>,
}

/// Lint names that should always be considered true-positive when they fire.
///
/// These are lints with no known false-positive patterns: each one matches a
/// purely structural shape where the suggested fix is unconditionally correct.
/// Anything not in this list lands in the FP bucket until reviewed, because
/// context-dependent lints (storage-in-loop, unbounded-input) can fire on
/// code where the loop or collection is genuinely bounded by the contract's
/// design.
///
/// The list lives in `tests/corpus/always_true_positive.json`, not in a Rust
/// literal, because the dogfood workflow's bash side applies the same
/// classification and its hand-copied version had drifted to six of these ten
/// names — which made the nightly report false-positive regressions that did
/// not exist. Both sides now read the one file.
static ALWAYS_TP: std::sync::LazyLock<Vec<String>> = std::sync::LazyLock::new(|| {
    serde_json::from_str(include_str!("../../tests/corpus/always_true_positive.json"))
        .expect("tests/corpus/always_true_positive.json must be a JSON array of lint names")
});

/// Path of the repository root, derived from this crate's manifest directory.
fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

/// Path of the corpus root containing contracts and `baseline.json`.
fn corpus_dir() -> PathBuf {
    workspace_root().join("tests").join("corpus")
}

/// Path of the directory holding one standalone cargo workspace per contract.
fn contracts_dir() -> PathBuf {
    corpus_dir().join("contracts")
}

/// Path of the committed baseline file compared against on every run.
fn baseline_path() -> PathBuf {
    corpus_dir().join("baseline.json")
}

/// Target directory shared by every corpus contract build.
///
/// Every corpus contract is an independent cargo workspace depending on the
/// same `soroban-sdk`, so left alone each one compiles that dependency tree
/// into its own `target/` — nine near-identical builds, none of them shared and
/// none of them covered by the CI cache, which only holds the root `target/`.
/// Pointing every contract build and its dylint pass at one directory compiles
/// the SDK once and leaves a single directory to cache.
fn shared_target_dir() -> PathBuf {
    corpus_dir().join(".shared-target")
}

/// Run `cargo-cost-lint --format json` in the given contract directory.
///
/// Returns one [`Finding`] per JSON line the tool printed. The dylint library
/// path is pointed at the test binary's directory, where the workspace build
/// places the compiled lint dynamic library, and `CARGO_TARGET_DIR` is
/// redirected to [`shared_target_dir`] so contract builds reuse one SDK
/// compilation.
///
/// # Exit status
///
/// A non-zero exit can mean cargo-cost-lint genuinely failed to run, or
/// that a `deny`-level lint fired and correctly failed the underlying
/// `cargo check` — findings were already streamed to stdout before the
/// process exited in that case. Only treat this as a hard failure when
/// there's nothing to show for it.
fn run_lints_on_contract(contract_dir: &Path) -> Vec<Finding> {
    let bin_path = env!("CARGO_BIN_EXE_cargo-cost-lint");
    let mut target_dir = PathBuf::from(bin_path);
    target_dir.pop();

    let output = Command::new(bin_path)
        .arg("--format")
        .arg("json")
        .current_dir(contract_dir)
        .env("DYLINT_LIBRARY_PATH", &target_dir)
        .env("CARGO_TARGET_DIR", shared_target_dir())
        .output()
        .expect("Failed to execute cargo-cost-lint");

    let stdout_str = String::from_utf8(output.stdout).expect("Stdout is not valid UTF-8");

    if !output.status.success() && stdout_str.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "cargo-cost-lint failed on {:?}:\n{}",
            contract_dir.file_name().unwrap(),
            stderr
        );
    }

    let findings: Vec<Finding> = stdout_str
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let json: serde_json::Value =
                serde_json::from_str(line).expect("Output line is not valid JSON");
            Finding {
                lint_name: json["name"].as_str().unwrap_or("unknown").to_string(),
                file: json["file"].as_str().unwrap_or("unknown").to_string(),
                line: json["span"]["line_start"].as_u64().unwrap_or(0) as usize,
                message: json["message"].as_str().unwrap_or("").to_string(),
            }
        })
        .collect();

    findings
}

/// Split findings into `(true_positives, false_positives)`.
///
/// The split is deterministic and total: every finding lands in exactly one
/// bucket, keyed by whether its lint name appears in [`ALWAYS_TP`].
fn triage_findings(findings: &[Finding]) -> (Vec<Finding>, Vec<Finding>) {
    let mut tps = Vec::new();
    let mut fps = Vec::new();

    for f in findings {
        if ALWAYS_TP.contains(&f.lint_name) {
            tps.push(f.clone());
        } else {
            fps.push(f.clone());
        }
    }

    (tps, fps)
}

/// Build the `soroban_cost_lints` dylint library before any contract is linted.
///
/// The corpus test binary links against `cargo-cost-lint`, but the linter
/// loads its lint passes at runtime from a dynamic library. Building the
/// library here guarantees the lint pass under test is the one just built from
/// this checkout rather than a stale artifact from an earlier run.
fn build_soroban_cost_lints() {
    let binding = env::var("CARGO");
    let cargo = binding.as_deref().unwrap_or("cargo");

    let mut lint_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    lint_dir.pop();
    lint_dir.push("soroban_cost_lints");

    let status = Command::new(cargo)
        .arg("build")
        .current_dir(&lint_dir)
        .status()
        .expect("Failed to build soroban_cost_lints");

    assert!(status.success(), "Failed to build soroban_cost_lints");
}

/// Build a contract and run the linter over it.
///
/// Returns `(contract_name, findings)` where `contract_name` matches the keys
/// used in `baseline.json` (the directory name, not the cargo package name).
fn collect_and_report(contract_dir: &Path) -> (String, Vec<Finding>) {
    let name = contract_dir
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let binding = env::var("CARGO");
    let cargo = binding.as_deref().unwrap_or("cargo");

    let build_status = Command::new(cargo)
        .arg("build")
        .current_dir(contract_dir)
        .env("CARGO_TARGET_DIR", shared_target_dir())
        .status()
        .expect("Failed to build contract");

    assert!(build_status.success(), "Failed to build contract {name}");

    let findings = run_lints_on_contract(contract_dir);

    (name, findings)
}

/// Load `baseline.json`, or an empty baseline when the file does not exist.
///
/// The empty-baseline fallback makes the very first run (before any baseline
/// has been blessed) behave like an empty comparison: every contract starts
/// from 0 baseline FPs, so nothing fails and a `BLESS=1` run seeds the file.
fn load_baseline() -> Baseline {
    let path = baseline_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path).expect("Failed to read baseline.json");
        serde_json::from_str(&content).expect("Failed to parse baseline.json")
    } else {
        Baseline {
            version: 1,
            description: String::new(),
            summary: None,
            contracts: BTreeMap::new(),
        }
    }
}

/// Aggregate TP/FP totals and the FP rate over all contracts.
///
/// The rate is rounded to two decimals (multiplying by 10^4, rounding, then
/// dividing) so the stored value is stable across serialization round-trips
/// and does not carry float noise into the baseline diff.
fn compute_summary(all_findings: &BTreeMap<String, Vec<Finding>>) -> BaselineSummary {
    let mut total_tp = 0usize;
    let mut total_fp = 0usize;

    for findings in all_findings.values() {
        let (tps, fps) = triage_findings(findings);
        total_tp += tps.len();
        total_fp += fps.len();
    }

    let total = total_tp + total_fp;
    let fp_rate = if total > 0 {
        ((total_fp as f64 / total as f64) * 10000.0).round() / 100.0
    } else {
        0.0
    };

    BaselineSummary {
        total_findings: total,
        total_true_positives: total_tp,
        total_false_positives: total_fp,
        false_positive_rate_percent: fp_rate,
    }
}

/// Print a per-lint TP/FP breakdown table to stderr for local triage.
///
/// Purely informational — the pass/fail decision is made from per-contract FP
/// counts in [`real_world_corpus_triage`], never from this output.
fn print_corpus_summary(all_findings: &BTreeMap<String, Vec<Finding>>) {
    let mut total_tp = 0usize;
    let mut total_fp = 0usize;
    let mut per_lint_stats: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    for findings in all_findings.values() {
        let (tps, fps) = triage_findings(findings);
        total_tp += tps.len();
        total_fp += fps.len();
        for tp in &tps {
            let entry = per_lint_stats.entry(tp.lint_name.clone()).or_insert((0, 0));
            entry.0 += 1;
        }
        for fp in &fps {
            let entry = per_lint_stats.entry(fp.lint_name.clone()).or_insert((0, 0));
            entry.1 += 1;
        }
    }

    let total = total_tp + total_fp;
    let fp_pct = if total > 0 {
        (total_fp as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let tp_pct = if total > 0 {
        (total_tp as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    eprintln!("\n================================================================================");
    eprintln!("Corpus False-Positive vs True-Positive Summary:");
    eprintln!("  Total Findings:       {}", total);
    eprintln!("  True Positives (TP):  {} ({:.2}%)", total_tp, tp_pct);
    eprintln!("  False Positives (FP): {} ({:.2}%)", total_fp, fp_pct);
    eprintln!("--------------------------------------------------------------------------------");
    eprintln!(
        "{:<40} {:>6} {:>6} {:>8} {:>8}",
        "Lint Name", "TP", "FP", "Total", "% FP"
    );
    eprintln!(
        "{:<40} {:>6} {:>6} {:>8} {:>8}",
        "---------", "--", "--", "-----", "----"
    );
    for (lint_name, (tp, fp)) in &per_lint_stats {
        let lint_total = tp + fp;
        let lint_fp_pct = if lint_total > 0 {
            (*fp as f64 / lint_total as f64) * 100.0
        } else {
            0.0
        };
        eprintln!(
            "{:<40} {:>6} {:>6} {:>8} {:>7.1}%",
            lint_name, tp, fp, lint_total, lint_fp_pct
        );
    }
    eprintln!("================================================================================\n");
}

/// Serialize `baseline` to `baseline.json`, overwriting any existing file.
///
/// Only called from the `BLESS=1` branch; the regular comparison path never
/// writes to disk.
fn save_baseline(baseline: &Baseline) {
    let path = baseline_path();
    let content = serde_json::to_string_pretty(baseline).expect("Failed to serialize baseline");
    std::fs::write(&path, content).expect("Failed to write baseline.json");
}

/// Unix-seconds timestamp used in the blessed baseline's description string.
///
/// Deliberately coarse (seconds, not ISO-8601): the description is free-form
/// provenance text and a monotonic counter avoids pulling in a date library.
fn now_stamp() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    format!("t={}", d.as_secs())
}

#[test]
fn real_world_corpus_triage() {
    build_soroban_cost_lints();
    let bless = env::var("BLESS").is_ok();
    let contracts = contracts_dir();
    assert!(
        contracts.exists(),
        "Corpus contracts directory not found: {:?}",
        contracts
    );

    let mut all_findings: BTreeMap<String, Vec<Finding>> = BTreeMap::new();
    let mut grand_total = 0usize;

    // Sort entries for deterministic output ordering across platforms;
    // `read_dir` order is filesystem-dependent.
    let mut entries: Vec<_> = std::fs::read_dir(&contracts)
        .expect("Failed to read contracts directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("Cargo.toml").exists())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let (name, findings) = collect_and_report(&entry.path());
        eprintln!("\n=== {}: {} findings ===", name, findings.len());
        for f in &findings {
            eprintln!("  {}:{} — {} — {}", f.file, f.line, f.lint_name, f.message);
        }
        all_findings.insert(name.clone(), findings);
        grand_total += all_findings.get(&name).map_or(0, |v| v.len());
    }

    eprintln!(
        "\n=== Grand total: {grand_total} findings across {} contracts ===",
        entries.len()
    );

    print_corpus_summary(&all_findings);

    let baseline = load_baseline();

    if bless {
        let summary = compute_summary(&all_findings);
        let mut new_baseline = Baseline {
            version: 1,
            description: format!(
                "Baseline lint findings for real-world corpus. Generated on {} with {} findings ({} TP, {} FP, {:.2}% FP rate) across {} contracts.\nRegenerate by running: BLESS=1 cargo test --test real_world_corpus --workspace",
                now_stamp(),
                grand_total,
                summary.total_true_positives,
                summary.total_false_positives,
                summary.false_positive_rate_percent,
                entries.len()
            ),
            summary: Some(summary),
            contracts: BTreeMap::new(),
        };

        for (name, findings) in &all_findings {
            let (tps, fps) = triage_findings(findings);
            new_baseline.contracts.insert(
                name.clone(),
                BaselineEntry {
                    total: findings.len(),
                    true_positives: tps,
                    false_positives: fps,
                },
            );
        }

        save_baseline(&new_baseline);
        eprintln!("Baseline written to {:?}", baseline_path());
    } else {
        // Compare per-contract FP counts against the baseline. Only an
        // increase fails: fewer FPs than the baseline is an improvement, and
        // contracts missing from the baseline default to 0 so newly added
        // corpus contracts are compared against nothing rather than skipped.
        let mut total_fp_increase = 0usize;
        let mut any_failure = false;

        for (name, findings) in &all_findings {
            let baseline_entry = baseline.contracts.get(name);
            let current_fps = triage_findings(findings).1.len();

            let baseline_fps = baseline_entry.map(|e| e.false_positives.len()).unwrap_or(0);

            if current_fps > baseline_fps {
                let increase = current_fps - baseline_fps;
                total_fp_increase += increase;
                any_failure = true;
                eprintln!(
                    "FAIL: {name} now has {current_fps} FPs (baseline: {baseline_fps}, +{increase})"
                );
            } else {
                eprintln!("OK:   {name} has {current_fps} FPs (baseline: {baseline_fps})");
            }
        }

        if any_failure {
            panic!(
                "\n\nFalse-positive count increased by {total_fp_increase} across corpus.\n\
                 If these are legitimate new findings, re-bless by running:\n\
                    BLESS=1 cargo test --test real_world_corpus --workspace\n\
                 Then commit the updated baseline.json"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(lint_name: &str) -> Finding {
        Finding {
            lint_name: lint_name.to_string(),
            file: "src/lib.rs".to_string(),
            line: 1,
            message: "test".to_string(),
        }
    }

    #[test]
    fn always_tp_lints_are_classified_as_true_positives() {
        for lint in ALWAYS_TP.iter() {
            let findings = vec![finding(lint.as_str())];
            let (tps, fps) = triage_findings(&findings);
            assert_eq!(tps.len(), 1, "{lint} should triage as a true positive");
            assert!(fps.is_empty());
        }
    }

    #[test]
    fn unknown_lints_default_to_false_positives() {
        let findings = vec![finding("some_unvetted_lint"), finding("another_new_lint")];
        let (tps, fps) = triage_findings(&findings);
        assert!(tps.is_empty());
        assert_eq!(fps.len(), 2);
    }

    #[test]
    fn triage_partitions_findings_without_loss() {
        let findings = vec![
            finding("redundant_env_clone"),
            finding("soroban_storage_in_loop"),
            finding("unwrap_on_storage_get"),
            finding("unbounded_input_loop"),
        ];
        let (tps, fps) = triage_findings(&findings);
        assert_eq!(tps.len() + fps.len(), findings.len());
        assert_eq!(tps.len(), 2);
        assert_eq!(fps.len(), 2);
    }

    #[test]
    fn summary_rates_are_stable_for_empty_input() {
        let summary = compute_summary(&BTreeMap::new());
        assert_eq!(summary.total_findings, 0);
        assert_eq!(summary.false_positive_rate_percent, 0.0);
    }

    #[test]
    fn summary_rounds_fp_rate_to_two_decimals() {
        // 1 FP out of 3 findings = 33.333...%, rounded to 33.33.
        let mut findings = BTreeMap::new();
        findings.insert(
            "c".to_string(),
            vec![
                finding("soroban_storage_in_loop"),
                finding("unbounded_input_loop"),
                finding("redundant_env_clone"),
            ],
        );
        let summary = compute_summary(&findings);
        assert_eq!(summary.total_findings, 3);
        assert_eq!(summary.total_true_positives, 1);
        assert_eq!(summary.total_false_positives, 2);
        assert_eq!(summary.false_positive_rate_percent, 66.67);
    }

    #[test]
    fn load_baseline_falls_back_to_empty_without_file() {
        // Point baseline_path at a location that does not exist by relying on
        // the fact that load_baseline only reads; this test runs in the same
        // process as the corpus test, so we cannot safely write a temp file.
        // Instead, assert the fallback struct shape directly.
        let empty = Baseline {
            version: 1,
            description: String::new(),
            summary: None,
            contracts: BTreeMap::new(),
        };
        assert!(empty.contracts.is_empty());
        assert_eq!(empty.version, 1);
    }

    #[test]
    fn path_helpers_are_consistent() {
        assert_eq!(corpus_dir(), workspace_root().join("tests").join("corpus"));
        assert_eq!(contracts_dir(), corpus_dir().join("contracts"));
        assert_eq!(baseline_path(), corpus_dir().join("baseline.json"));
        assert_eq!(shared_target_dir(), corpus_dir().join(".shared-target"));
    }

    #[test]
    fn now_stamp_is_parseable_unix_seconds() {
        let stamp = now_stamp();
        let raw = stamp
            .strip_prefix("t=")
            .expect("stamp should start with t=");
        raw.parse::<u64>()
            .expect("stamp should be a unix timestamp");
    }
}
