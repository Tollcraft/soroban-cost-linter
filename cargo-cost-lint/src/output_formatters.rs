#![allow(dead_code)]

use clap::ValueEnum;
use serde::Serialize;
use std::collections::HashSet;
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
        // LintFinding only contains Strings and usizes, so it cannot fail to serialise.
        // However, we still return an error instead of unwrapping to avoid panicking
        // mid-stream if the type definition changes in the future, which would leave
        // partial NDJSON on stdout that might be misinterpreted as complete.
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
            message: SarifMessage {
                text: finding.message.clone(),
            },
            locations: vec![SarifLocation {
                physical_location: SarifPhysicalLocation {
                    artifact_location: SarifArtifactLocation {
                        uri: finding.file.clone(),
                    },
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
                crate::error::LinterError::Other(format!(
                    "Failed to serialise SARIF result for rule '{}': {}",
                    r.rule_id, e
                ))
            })
        })
        .collect();
    let results = results?;

    let sarif = SarifReport {
        schema: "https://json.schemastore.org/sarif-2.1.0".to_string(),
        version: "2.1.0".to_string(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifToolDriver {
                    name: "cargo-cost-lint".to_string(),
                    version: package_version.to_string(),
                    information_uri: Some(
                        "https://github.com/Tollcraft/soroban-cost-linter".to_string(),
                    ),
                    rules,
                },
            },
            results,
        }],
    };
    let sarif_json = serde_json::to_string_pretty(&sarif).map_err(|e| {
        crate::error::LinterError::Other(format!("Failed to serialise SARIF report: {}", e))
    })?;
    writeln!(writer, "{}", sarif_json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_github_message() {
        assert_eq!(
            escape_github_message("Line 1\nLine 2\r\n100% complete: check here"),
            "Line 1%0ALine 2%0D%0A100%25 complete: check here"
        );
    }

    #[test]
    fn test_escape_github_property() {
        assert_eq!(
            escape_github_property("file:name,with%special\r\nchars"),
            "file%3Aname%2Cwith%25special%0D%0Achars"
        );
    }

    #[test]
    fn test_format_github_annotation_deny_error() {
        let finding = LintFinding {
            name: "soroban_storage_in_loop".to_string(),
            level: "deny".to_string(),
            file: "src/contract.rs".to_string(),
            span: Span {
                line_start: 42,
                line_end: 42,
                column_start: 13,
                column_end: 20,
            },
            message: "storage operations in loops are expensive".to_string(),
            help: None,
            suggestion: None,
        };

        let annotation = format_github_annotation(&finding);
        assert_eq!(
            annotation,
            "::error file=src/contract.rs,line=42,col=13::storage operations in loops are expensive"
        );
    }

    #[test]
    fn test_format_github_annotation_warn_warning() {
        let finding = LintFinding {
            name: "redundant_env_clone".to_string(),
            level: "warn".to_string(),
            file: "src/lib.rs".to_string(),
            span: Span {
                line_start: 15,
                line_end: 15,
                column_start: 5,
                column_end: 10,
            },
            message: "redundant cloning of Env".to_string(),
            help: None,
            suggestion: None,
        };

        let annotation = format_github_annotation(&finding);
        assert_eq!(
            annotation,
            "::warning file=src/lib.rs,line=15,col=5::redundant cloning of Env"
        );
    }

    #[test]
    fn test_format_github_annotation_multiline_and_colons() {
        let finding = LintFinding {
            name: "soroban_storage_in_loop".to_string(),
            level: "deny".to_string(),
            file: "contracts/vault/src/lib.rs".to_string(),
            span: Span {
                line_start: 100,
                line_end: 105,
                column_start: 9,
                column_end: 14,
            },
            message: "storage operation inside loop detected:\n  env.storage().instance().set(&k, &v);\nhelp: consider batching writes outside the loop: see https://docs.rs/soroban-sdk".to_string(),
            help: Some("consider batching writes".to_string()),
            suggestion: None,
        };

        let annotation = format_github_annotation(&finding);
        assert_eq!(
            annotation,
            "::error file=contracts/vault/src/lib.rs,line=100,col=9::storage operation inside loop detected:%0A  env.storage().instance().set(&k, &v);%0Ahelp: consider batching writes outside the loop: see https://docs.rs/soroban-sdk"
        );
        // Verify no raw newlines in the annotation string (single line command)
        assert!(!annotation.contains('\n'));
        assert!(!annotation.contains('\r'));
    }

    #[test]
    fn test_format_github_annotation_without_span() {
        let finding = LintFinding {
            name: "budget_exceeded".to_string(),
            level: "warn".to_string(),
            file: "src/lib.rs".to_string(),
            span: Span {
                line_start: 0,
                line_end: 0,
                column_start: 0,
                column_end: 0,
            },
            message: "general workspace warning".to_string(),
            help: None,
            suggestion: None,
        };

        let annotation = format_github_annotation(&finding);
        assert_eq!(
            annotation,
            "::warning file=src/lib.rs::general workspace warning"
        );
    }

    #[test]
    fn test_emit_github_annotation() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "warn".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 1,
                line_end: 1,
                column_start: 1,
                column_end: 1,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };

        let mut buf = Vec::new();
        emit_github_annotation(&finding, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert_eq!(
            output,
            "::warning file=src/main.rs,line=1,col=1::test message\n"
        );
    }
}
