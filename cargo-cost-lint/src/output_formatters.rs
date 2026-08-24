// src/output_formatters.rs
use clap::ValueEnum;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::io::{self, Write};

/// Output format for lint results.
#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable text output (default).
    Text,
    /// Newline-delimited JSON objects.
    Json,
    /// SARIF 2.1.0 JSON report.
    Sarif,
}

/// Source-location span for a lint finding.
#[derive(Serialize, Debug, Clone)]
pub struct Span {
    pub line_start: usize,
    pub line_end: usize,
    pub column_start: usize,
    pub column_end: usize,
}

/// A single lint finding produced by `cargo dylint`.
#[derive(Serialize, Debug, Clone)]
pub struct LintFinding {
    pub name: String,
    pub level: String,
    pub file: String,
    pub span: Span,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// SARIF 2.1.0 report root.
#[derive(Serialize)]
pub struct SarifReport {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub runs: Vec<SarifRun>,
}

/// A single SARIF run (one invocation of the linter).
#[derive(Serialize)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<serde_json::Value>,
}

/// Tool metadata for SARIF output.
#[derive(Serialize)]
pub struct SarifTool {
    pub driver: SarifToolDriver,
}

/// Tool-driver metadata for SARIF output.
#[allow(non_snake_case)]
#[derive(Serialize)]
pub struct SarifToolDriver {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "informationUri")]
    pub information_uri: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<serde_json::Value>,
}

/// A single SARIF result (one lint finding).
#[derive(Serialize)]
pub struct SarifResult {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub level: String,
    pub message: SarifMessage,
    pub locations: Vec<SarifLocation>,
}

/// SARIF message text.
#[derive(Serialize)]
pub struct SarifMessage {
    pub text: String,
}

/// SARIF location referencing a physical file.
#[derive(Serialize)]
pub struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    pub physical_location: SarifPhysicalLocation,
}

/// SARIF physical location in a file.
#[derive(Serialize)]
pub struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    pub artifact_location: SarifArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<SarifRegion>,
}

/// SARIF artifact location (file URI).
#[derive(Serialize)]
pub struct SarifArtifactLocation {
    pub uri: String,
}

/// SARIF region (line/column range within a file).
#[derive(Serialize)]
pub struct SarifRegion {
    #[serde(rename = "startLine")]
    pub start_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "startColumn")]
    pub start_column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "endLine")]
    pub end_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "endColumn")]
    pub end_column: Option<usize>,
}

/// Print a single finding according to the CLI format.
/// Returns true if anything was printed.
pub fn handle_finding<W: Write>(
    cli: &crate::Cli,
    finding: &LintFinding,
    findings_acc: &mut Vec<LintFinding>,
    writer: &mut W,
) -> crate::error::LinterResult<bool> {
    // Store the finding for later SARIF generation.
    findings_acc.push(finding.clone());
    if cli.format == OutputFormat::Json {
        // LintFinding only contains Strings and usizes, so it cannot fail to serialise.
        // However, we still return an error instead of unwrapping to avoid panicking
        // mid-stream if the type definition changes in the future, which would leave
        // partial NDJSON on stdout that might be misinterpreted as complete.
        let json_str = serde_json::to_string(finding)
            .map_err(|e| crate::error::LinterError::Other(format!("Failed to serialise finding '{}': {}", finding.name, e)))?;
        writeln!(writer, "{}", json_str)?;
        return Ok(true);
    }
    // For non‑SARIF formats we render the diagnostic message.
    if cli.format != OutputFormat::Sarif {
        // Use the rendered field if available; fallback to the raw message.
        // This matches the behaviour in the original main.rs.
        // The caller should have supplied the rendered string via the JSON message.
        // Here we simply output the message field (which already contains the rendered text).
        write!(writer, "{}", finding.message)?;
    }
    Ok(true)
}

/// Emit a SARIF report for all collected findings.
pub fn emit_sarif<W: Write>(
    findings: &[LintFinding],
    writer: &mut W,
) -> crate::error::LinterResult<()> {
    let package_version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.1.0");
    let mut rules: Vec<serde_json::Value> = Vec::new();
    let mut seen_rules: HashSet<String> = HashSet::new();
    let mut sarif_results: Vec<SarifResult> = Vec::new();

    for finding in findings {
        if seen_rules.insert(finding.name.clone()) {
            rules.push(serde_json::json!({
                "id": finding.name,
                "shortDescription": { "text": finding.message }
            }));
        }
        let level = match finding.level.as_str() {
            "error" | "deny" => "error",
            _ => "warning",
        };
        let region = if finding.span.line_start > 0 {
            Some(SarifRegion {
                start_line: finding.span.line_start,
                start_column: Some(finding.span.column_start),
                end_line: Some(finding.span.line_end),
                end_column: Some(finding.span.column_end),
            })
        } else {
            None
        };
        sarif_results.push(SarifResult {
            rule_id: finding.name.clone(),
            level: level.to_string(),
            message: SarifMessage { text: finding.message.clone() },
            locations: vec![SarifLocation {
                physical_location: SarifPhysicalLocation {
                    artifact_location: SarifArtifactLocation { uri: finding.file.clone() },
                    region,
                },
            }],
        });
    }

    // SarifResult and SarifReport only contain standard types that cannot fail to serialise.
    // We map the error and return it instead of unwrapping to guarantee the linter exits
    // gracefully if these types are ever changed to include fallible data structures.
    let results: Result<Vec<serde_json::Value>, crate::error::LinterError> = sarif_results
        .iter()
        .map(|r| {
            serde_json::to_value(r).map_err(|e| {
                crate::error::LinterError::Other(format!("Failed to serialise SARIF result for rule '{}': {}", r.rule_id, e))
            })
        })
        .collect();
    let results = results?;

    let sarif = SarifReport {
        schema: "https://json.schemastore.org/sarif-2.1.0".to_string(),
        version: "2.1.0".to_string(),
        runs: vec![SarifRun { tool: SarifTool { driver: SarifToolDriver {
            name: "cargo-cost-lint".to_string(),
            version: package_version.to_string(),
            information_uri: Some("https://github.com/Tollcraft/soroban-cost-linter".to_string()),
            rules,
        }}, results }],
    };
    let sarif_json = serde_json::to_string_pretty(&sarif)
        .map_err(|e| crate::error::LinterError::Other(format!("Failed to serialise SARIF report: {}", e)))?;
    writeln!(writer, "{}", sarif_json)?;
    Ok(())
}

// The suggestion extraction and fix‑application logic remain in the main crate.
