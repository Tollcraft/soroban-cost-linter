#![allow(dead_code)]

use clap::ValueEnum;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;

/// Output format for lint results.
#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable text output (default).
    Text,
    /// Newline-delimited JSON objects.
    Json,
    /// SARIF 2.1.0 JSON report.
    Sarif,
    /// GitHub Actions workflow command annotations.
    Github,
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

/// Escapes special characters for GitHub Actions workflow command message bodies.
///
/// According to GitHub Actions specification:
/// - `%` is escaped as `%25`
/// - `\r` is escaped as `%0D`
/// - `\n` is escaped as `%0A`
pub fn escape_github_message(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Escapes special characters for GitHub Actions workflow command property values.
///
/// According to GitHub Actions specification:
/// - `%` is escaped as `%25`
/// - `\r` is escaped as `%0D`
/// - `\n` is escaped as `%0A`
/// - `:` is escaped as `%3A`
/// - `,` is escaped as `%2C`
pub fn escape_github_property(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
        .replace(':', "%3A")
        .replace(',', "%2C")
}

/// Format a single finding as a GitHub Actions workflow command annotation.
pub fn format_github_annotation(finding: &LintFinding) -> String {
    let severity = match finding.level.as_str() {
        "error" | "deny" => "error",
        _ => "warning",
    };

    let escaped_message = escape_github_message(&finding.message);

    // Normalize file path: make relative to current directory if absolute, and use forward slashes.
    let rel_file = if let Ok(current_dir) = std::env::current_dir() {
        let p = std::path::Path::new(&finding.file);
        if let Ok(stripped) = p.strip_prefix(&current_dir) {
            stripped.to_string_lossy().replace('\\', "/")
        } else {
            finding.file.replace('\\', "/")
        }
    } else {
        finding.file.replace('\\', "/")
    };

    let escaped_file = escape_github_property(&rel_file);

    if finding.span.line_start > 0 {
        if finding.span.column_start > 0 {
            format!(
                "::{} file={},line={},col={}::{}",
                severity,
                escaped_file,
                finding.span.line_start,
                finding.span.column_start,
                escaped_message
            )
        } else {
            format!(
                "::{} file={},line={}::{}",
                severity, escaped_file, finding.span.line_start, escaped_message
            )
        }
    } else if !escaped_file.is_empty() {
        format!("::{} file={}::{}", severity, escaped_file, escaped_message)
    } else {
        format!("::{}::{}", severity, escaped_message)
    }
}

/// Emit a GitHub Actions workflow command annotation for a finding.
pub fn emit_github_annotation<W: Write>(
    finding: &LintFinding,
    writer: &mut W,
) -> crate::error::LinterResult<()> {
    let annotation = format_github_annotation(finding);
    writeln!(writer, "{}", annotation)?;
    Ok(())
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
        let json_str = serde_json::to_string(finding).map_err(|e| {
            crate::error::LinterError::Other(format!(
                "Failed to serialise finding '{}': {}",
                finding.name, e
            ))
        })?;
        writeln!(writer, "{}", json_str)?;
        return Ok(true);
    }
    if cli.format == OutputFormat::Github {
        emit_github_annotation(finding, writer)?;
        return Ok(true);
    }
    // For non-SARIF formats we render the diagnostic message.
    if cli.format != OutputFormat::Sarif {
        let formatted = format_diagnostic(finding);
        writeln!(writer, "{}", formatted)?;
        return Ok(true);
    }
    Ok(false)
}

/// Print a summary of findings at the end of a run.
/// For Text format, shows total count, breakdown by lint name (ordered by count descending), and breakdown by severity.
/// For zero findings, produces a clear success line.
/// Omitted for machine-readable formats (Json, Sarif, Github).
pub fn print_findings_summary<W: Write>(
    format: &OutputFormat,
    findings: &[LintFinding],
    writer: &mut W,
) -> crate::error::LinterResult<()> {
    if matches!(
        format,
        OutputFormat::Json | OutputFormat::Sarif | OutputFormat::Github
    ) {
        return Ok(());
    }

    writeln!(writer)?;
    if findings.is_empty() {
        writeln!(writer, "✓ No cost lints found. Clean workspace!")?;
    } else {
        let total = findings.len();
        writeln!(
            writer,
            "Found {} cost lint finding{}:",
            total,
            if total == 1 { "" } else { "s" }
        )?;

        // Breakdown by lint name
        let mut lint_counts: HashMap<String, usize> = HashMap::new();
        // Breakdown by severity
        let mut severity_counts: HashMap<String, usize> = HashMap::new();

        for f in findings {
            *lint_counts.entry(f.name.clone()).or_insert(0) += 1;
            *severity_counts.entry(f.level.clone()).or_insert(0) += 1;
        }

        // Sort lints by count descending, then name ascending for stability
        let mut sorted_lints: Vec<(String, usize)> = lint_counts.into_iter().collect();
        sorted_lints.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        writeln!(writer, "  By lint name:")?;
        for (name, count) in &sorted_lints {
            writeln!(writer, "    - {}: {}", name, count)?;
        }

        // Sort severities for predictable output
        let mut sorted_severities: Vec<(String, usize)> = severity_counts.into_iter().collect();
        sorted_severities.sort_by(|a, b| b.0.cmp(&a.0));

        writeln!(writer, "  By severity:")?;
        for (level, count) in &sorted_severities {
            writeln!(writer, "    - {}: {}", level, count)?;
        }
    }

    Ok(())
}

/// Format a finding as a human-readable text diagnostic.
pub fn format_diagnostic(finding: &LintFinding) -> String {
    let severity_prefix = match finding.level.as_str() {
        "error" | "deny" => "error",
        "warning" | "warn" => "warning",
        _ => "note",
    };

    let location = if !finding.file.is_empty() && finding.span.line_start > 0 {
        format!(
            "{}:{}:{}",
            finding.file, finding.span.line_start, finding.span.column_start
        )
    } else if !finding.file.is_empty() {
        finding.file.clone()
    } else {
        "unknown location".to_string()
    };

    let mut output = format!(
        "{}: [{}] {}\n  --> {}\n  = note: {}",
        severity_prefix, finding.name, finding.message, location, finding.message
    );

    if let Some(ref help) = finding.help {
        output.push_str(&format!("\n  = help: {}", help));
    }

    if let Some(ref suggestion) = finding.suggestion {
        output.push_str(&format!("\n  = suggestion: {}", suggestion));
    }

    output
}

/// Generate SARIF 2.1.0 JSON report from accumulated findings.
pub fn generate_sarif_report(findings: &[LintFinding]) -> String {
    let results: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| {
            let level = match f.level.as_str() {
                "error" | "deny" => "error",
                "warning" | "warn" => "warning",
                _ => "note",
            };

            let uri = if let Ok(current_dir) = std::env::current_dir() {
                let p = std::path::Path::new(&f.file);
                if let Ok(stripped) = p.strip_prefix(&current_dir) {
                    format!("file:///{}", stripped.to_string_lossy().replace('\\', "/"))
                } else {
                    format!("file:///{}", f.file.replace('\\', "/"))
                }
            } else {
                format!("file:///{}", f.file.replace('\\', "/"))
            };

            let region = if f.span.line_start > 0 {
                Some(SarifRegion {
                    start_line: f.span.line_start,
                    start_column: if f.span.column_start > 0 {
                        Some(f.span.column_start)
                    } else {
                        None
                    },
                    end_line: if f.span.line_end > 0 {
                        Some(f.span.line_end)
                    } else {
                        None
                    },
                    end_column: if f.span.column_end > 0 {
                        Some(f.span.column_end)
                    } else {
                        None
                    },
                })
            } else {
                None
            };

            let sarif_result = SarifResult {
                rule_id: f.name.clone(),
                level: level.to_string(),
                message: SarifMessage {
                    text: f.message.clone(),
                },
                locations: vec![SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation { uri },
                        region,
                    },
                }],
            };

            serde_json::to_value(sarif_result).unwrap_or(serde_json::Value::Null)
        })
        .collect();

    let report = SarifReport {
        schema: "https://schemastore.azurewebsites.net/schemas/json/sarif-2.1.0-rtm.5.json"
            .to_string(),
        version: "2.1.0".to_string(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifToolDriver {
                    name: "cargo-cost-lint".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    information_uri: Some(
                        "https://github.com/Tollcraft/soroban-cost-linter".to_string(),
                    ),
                    rules: vec![],
                },
            },
            results,
        }],
    };

    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_zero_findings() {
        let mut buf = Vec::new();
        print_findings_summary(&OutputFormat::Text, &[], &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("No cost lints found. Clean workspace!"));
    }

    #[test]
    fn test_summary_populated_findings() {
        let findings = vec![
            LintFinding {
                name: "lint_b".to_string(),
                level: "warning".to_string(),
                file: "src/lib.rs".to_string(),
                span: Span {
                    line_start: 1,
                    line_end: 1,
                    column_start: 1,
                    column_end: 5,
                },
                message: "msg b".to_string(),
                help: None,
                suggestion: None,
            },
            LintFinding {
                name: "lint_a".to_string(),
                level: "warning".to_string(),
                file: "src/lib.rs".to_string(),
                span: Span {
                    line_start: 2,
                    line_end: 2,
                    column_start: 1,
                    column_end: 5,
                },
                message: "msg a1".to_string(),
                help: None,
                suggestion: None,
            },
            LintFinding {
                name: "lint_a".to_string(),
                level: "error".to_string(),
                file: "src/lib.rs".to_string(),
                span: Span {
                    line_start: 3,
                    line_end: 3,
                    column_start: 1,
                    column_end: 5,
                },
                message: "msg a2".to_string(),
                help: None,
                suggestion: None,
            },
        ];

        let mut buf = Vec::new();
        print_findings_summary(&OutputFormat::Text, &findings, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("Found 3 cost lint findings:"));
        assert!(output.contains("By lint name:"));
        assert!(output.contains("By severity:"));

        // Check sorting by count descending (lint_a has 2, lint_b has 1)
        let idx_a = output.find("lint_a: 2").unwrap();
        let idx_b = output.find("lint_b: 1").unwrap();
        assert!(
            idx_a < idx_b,
            "lint_a should appear before lint_b due to higher count"
        );
    }

    #[test]
    fn test_summary_suppression_under_machine_formats() {
        let findings = vec![LintFinding {
            name: "lint_a".to_string(),
            level: "warning".to_string(),
            file: "src/lib.rs".to_string(),
            span: Span {
                line_start: 1,
                line_end: 1,
                column_start: 1,
                column_end: 5,
            },
            message: "msg".to_string(),
            help: None,
            suggestion: None,
        }];

        for fmt in &[
            OutputFormat::Json,
            OutputFormat::Sarif,
            OutputFormat::Github,
        ] {
            let mut buf = Vec::new();
            print_findings_summary(fmt, &findings, &mut buf).unwrap();
            let output = String::from_utf8(buf).unwrap();
            assert!(
                output.is_empty(),
                "Summary should be omitted for format {:?}",
                fmt
            );
        }
    }
}
