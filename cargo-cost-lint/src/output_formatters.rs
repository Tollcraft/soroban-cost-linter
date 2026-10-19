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
    pub results: Vec<SarifResult>,
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
    pub rules: Vec<SarifRule>,
}

/// A single SARIF rule (one registered lint).
#[derive(Serialize)]
pub struct SarifRule {
    pub id: String,
    #[serde(rename = "shortDescription")]
    pub short_description: SarifRuleShortDescription,
}

/// SARIF rule short-description container.
#[derive(Serialize)]
pub struct SarifRuleShortDescription {
    pub text: String,
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

/// Store a finding for SARIF generation and print it for human/machine
/// readable formats (Text, Json, Github).
pub fn handle_finding<W: Write>(
    cli: &crate::Cli,
    finding: &LintFinding,
    findings_acc: &mut Vec<LintFinding>,
    writer: &mut W,
) -> crate::error::LinterResult<()> {
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
        return Ok(());
    }
    if cli.format == OutputFormat::Github {
        emit_github_annotation(finding, writer)?;
        return Ok(());
    }
    // For non-SARIF formats we render the diagnostic message.
    if cli.format != OutputFormat::Sarif {
        let formatted = format_diagnostic(finding);
        writeln!(writer, "{}", formatted)?;
        return Ok(());
    }
    Ok(())
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
    let results: Vec<SarifResult> = findings
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
                    stripped.to_string_lossy().replace('\\', "/")
                } else if p.is_absolute() {
                    format!("file://{}", p.to_string_lossy().replace('\\', "/"))
                } else {
                    f.file.replace('\\', "/")
                }
            } else {
                f.file.replace('\\', "/")
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

            SarifResult {
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
            }
        })
        .collect();

    // Collect rule names in a `BTreeSet` so `tool.driver.rules` is emitted in
    // a deterministic (lexicographically sorted) order across runs. A
    // `HashSet` here iterates in a hash-randomised order, which permuted the
    // rules array between processes (#876). Deduplication semantics are
    // unchanged: one rule per distinct lint name.
    let unique_rules: std::collections::BTreeSet<&str> =
        findings.iter().map(|f| f.name.as_str()).collect();
    let rules: Vec<SarifRule> = unique_rules
        .into_iter()
        .map(|name| SarifRule {
            id: name.to_string(),
            short_description: SarifRuleShortDescription {
                text: format!("Cost lint rule: {}", name),
            },
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
                    rules,
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

    // --- Additional edge-case tests ---

    #[test]
    fn test_summary_single_finding() {
        let findings = vec![LintFinding {
            name: "lint_a".to_string(),
            level: "error".to_string(),
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

        let mut buf = Vec::new();
        print_findings_summary(&OutputFormat::Text, &findings, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Found 1 cost lint finding:"));
        assert!(output.contains("lint_a: 1"));
    }

    #[test]
    fn test_summary_tie_breaking_by_name() {
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
                message: "msg".to_string(),
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
                message: "msg".to_string(),
                help: None,
                suggestion: None,
            },
        ];

        let mut buf = Vec::new();
        print_findings_summary(&OutputFormat::Text, &findings, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        // Both have count 1, so they should be sorted by name ascending
        let idx_a = output.find("lint_a: 1").unwrap();
        let idx_b = output.find("lint_b: 1").unwrap();
        assert!(idx_a < idx_b, "lint_a should come before lint_b");
    }

    #[test]
    fn test_summary_severity_sorting() {
        let findings = vec![
            LintFinding {
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
            },
            LintFinding {
                name: "lint_b".to_string(),
                level: "error".to_string(),
                file: "src/lib.rs".to_string(),
                span: Span {
                    line_start: 2,
                    line_end: 2,
                    column_start: 1,
                    column_end: 5,
                },
                message: "msg".to_string(),
                help: None,
                suggestion: None,
            },
        ];

        let mut buf = Vec::new();
        print_findings_summary(&OutputFormat::Text, &findings, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        // Severities sorted descending alphabetically: "warning" before "error"
        let idx_error = output.find("error: 1").unwrap();
        let idx_warning = output.find("warning: 1").unwrap();
        assert!(idx_warning < idx_error);
    }

    #[test]
    fn test_escape_github_message_percent() {
        assert_eq!(escape_github_message("100%"), "100%25");
    }

    #[test]
    fn test_escape_github_message_newline() {
        assert_eq!(escape_github_message("line1\nline2"), "line1%0Aline2");
    }

    #[test]
    fn test_escape_github_message_carriage_return() {
        assert_eq!(escape_github_message("line1\rline2"), "line1%0Dline2");
    }

    #[test]
    fn test_escape_github_message_all_special() {
        assert_eq!(escape_github_message("100%\r\n"), "100%25%0D%0A");
    }

    #[test]
    fn test_escape_github_message_no_special() {
        assert_eq!(escape_github_message("hello world"), "hello world");
    }

    #[test]
    fn test_escape_github_property_colon() {
        assert_eq!(escape_github_property("file.rs:10"), "file.rs%3A10");
    }

    #[test]
    fn test_escape_github_property_comma() {
        assert_eq!(escape_github_property("a,b,c"), "a%2Cb%2Cc");
    }

    #[test]
    fn test_escape_github_property_all_special() {
        assert_eq!(
            escape_github_property("path:10,20%\r\n"),
            "path%3A10%2C20%25%0D%0A"
        );
    }

    #[test]
    fn test_format_github_annotation_error_level() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };
        let ann = format_github_annotation(&finding);
        assert!(ann.starts_with("::error file="));
        assert!(ann.contains("line=10"));
        assert!(ann.contains("col=5"));
        assert!(ann.contains("::test message"));
    }

    #[test]
    fn test_format_github_annotation_deny_level() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "deny".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };
        let ann = format_github_annotation(&finding);
        assert!(ann.starts_with("::error file="));
    }

    #[test]
    fn test_format_github_annotation_warning_level() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "warning".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };
        let ann = format_github_annotation(&finding);
        assert!(ann.starts_with("::warning file="));
    }

    #[test]
    fn test_format_github_annotation_no_line() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 0,
                line_end: 0,
                column_start: 0,
                column_end: 0,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };
        let ann = format_github_annotation(&finding);
        assert!(ann.starts_with("::error file="));
        assert!(!ann.contains("line="));
    }

    #[test]
    fn test_format_github_annotation_no_file() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: String::new(),
            span: Span {
                line_start: 0,
                line_end: 0,
                column_start: 0,
                column_end: 0,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };
        let ann = format_github_annotation(&finding);
        assert!(ann.starts_with("::error::"));
    }

    #[test]
    fn test_format_github_annotation_escapes_message() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 1,
                line_end: 1,
                column_start: 1,
                column_end: 5,
            },
            message: "100% failure\n".to_string(),
            help: None,
            suggestion: None,
        };
        let ann = format_github_annotation(&finding);
        assert!(ann.contains("100%25 failure%0A"));
    }

    #[test]
    fn test_emit_github_annotation_writes_to_writer() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 1,
                line_end: 1,
                column_start: 1,
                column_end: 5,
            },
            message: "test".to_string(),
            help: None,
            suggestion: None,
        };
        let mut buf = Vec::new();
        emit_github_annotation(&finding, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("::error file="));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn test_format_diagnostic_error_level() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };
        let output = format_diagnostic(&finding);
        assert!(output.contains("error: [test_lint] test message"));
        assert!(output.contains("src/main.rs:10:5"));
    }

    #[test]
    fn test_format_diagnostic_deny_level() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "deny".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };
        let output = format_diagnostic(&finding);
        assert!(output.contains("error: [test_lint]"));
    }

    #[test]
    fn test_format_diagnostic_warning_level() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "warning".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };
        let output = format_diagnostic(&finding);
        assert!(output.contains("warning: [test_lint]"));
    }

    #[test]
    fn test_format_diagnostic_warn_level() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "warn".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };
        let output = format_diagnostic(&finding);
        assert!(output.contains("warning: [test_lint]"));
    }

    #[test]
    fn test_format_diagnostic_note_level() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "note".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };
        let output = format_diagnostic(&finding);
        assert!(output.contains("note: [test_lint]"));
    }

    #[test]
    fn test_format_diagnostic_unknown_level() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "something".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };
        let output = format_diagnostic(&finding);
        assert!(output.contains("note: [test_lint]"));
    }

    #[test]
    fn test_format_diagnostic_no_file() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: String::new(),
            span: Span {
                line_start: 0,
                line_end: 0,
                column_start: 0,
                column_end: 0,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };
        let output = format_diagnostic(&finding);
        assert!(output.contains("unknown location"));
    }

    #[test]
    fn test_format_diagnostic_file_no_line() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 0,
                line_end: 0,
                column_start: 0,
                column_end: 0,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };
        let output = format_diagnostic(&finding);
        assert!(output.contains("src/main.rs"));
        assert!(!output.contains("src/main.rs:0"));
    }

    #[test]
    fn test_format_diagnostic_with_help() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: Some("try this instead".to_string()),
            suggestion: None,
        };
        let output = format_diagnostic(&finding);
        assert!(output.contains("= help: try this instead"));
    }

    #[test]
    fn test_format_diagnostic_with_suggestion() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: Some("use a different approach".to_string()),
        };
        let output = format_diagnostic(&finding);
        assert!(output.contains("= suggestion: use a different approach"));
    }

    #[test]
    fn test_format_diagnostic_with_help_and_suggestion() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: Some("help text".to_string()),
            suggestion: Some("suggestion text".to_string()),
        };
        let output = format_diagnostic(&finding);
        assert!(output.contains("= help: help text"));
        assert!(output.contains("= suggestion: suggestion text"));
    }

    #[test]
    fn test_generate_sarif_report_empty() {
        let report = generate_sarif_report(&[]);
        let value: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert_eq!(value["version"], "2.1.0");
        assert_eq!(value["runs"][0]["results"].as_array().unwrap().len(), 0);
        assert!(value["runs"][0]["tool"]["driver"].get("rules").is_none());
    }

    #[test]
    fn test_generate_sarif_report_single_finding() {
        let findings = vec![LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        }];

        let report = generate_sarif_report(&findings);
        let value: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert_eq!(value["version"], "2.1.0");
        assert_eq!(value["runs"][0]["results"].as_array().unwrap().len(), 1);
        assert_eq!(value["runs"][0]["results"][0]["ruleId"], "test_lint");
        assert_eq!(value["runs"][0]["results"][0]["level"], "error");
        assert_eq!(
            value["runs"][0]["results"][0]["message"]["text"],
            "test message"
        );
        assert_eq!(
            value["runs"][0]["tool"]["driver"]["rules"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            value["runs"][0]["tool"]["driver"]["rules"][0]["id"],
            "test_lint"
        );
    }

    #[test]
    fn test_generate_sarif_report_multiple_findings_same_rule() {
        let findings = vec![
            LintFinding {
                name: "test_lint".to_string(),
                level: "error".to_string(),
                file: "src/main.rs".to_string(),
                span: Span {
                    line_start: 10,
                    line_end: 12,
                    column_start: 5,
                    column_end: 15,
                },
                message: "msg1".to_string(),
                help: None,
                suggestion: None,
            },
            LintFinding {
                name: "test_lint".to_string(),
                level: "warning".to_string(),
                file: "src/lib.rs".to_string(),
                span: Span {
                    line_start: 20,
                    line_end: 22,
                    column_start: 3,
                    column_end: 10,
                },
                message: "msg2".to_string(),
                help: None,
                suggestion: None,
            },
        ];

        let report = generate_sarif_report(&findings);
        let value: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert_eq!(value["runs"][0]["results"].as_array().unwrap().len(), 2);
        // Rules should be deduplicated
        assert_eq!(
            value["runs"][0]["tool"]["driver"]["rules"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn test_generate_sarif_report_deny_level_maps_to_error() {
        let findings = vec![LintFinding {
            name: "test_lint".to_string(),
            level: "deny".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        }];

        let report = generate_sarif_report(&findings);
        let value: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert_eq!(value["runs"][0]["results"][0]["level"], "error");
    }

    #[test]
    fn test_generate_sarif_report_warn_level_maps_to_warning() {
        let findings = vec![LintFinding {
            name: "test_lint".to_string(),
            level: "warn".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        }];

        let report = generate_sarif_report(&findings);
        let value: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert_eq!(value["runs"][0]["results"][0]["level"], "warning");
    }

    #[test]
    fn test_generate_sarif_report_unknown_level_maps_to_note() {
        let findings = vec![LintFinding {
            name: "test_lint".to_string(),
            level: "info".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        }];

        let report = generate_sarif_report(&findings);
        let value: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert_eq!(value["runs"][0]["results"][0]["level"], "note");
    }

    #[test]
    fn test_generate_sarif_report_no_span() {
        let findings = vec![LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 0,
                line_end: 0,
                column_start: 0,
                column_end: 0,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        }];

        let report = generate_sarif_report(&findings);
        let value: serde_json::Value = serde_json::from_str(&report).unwrap();
        let region = &value["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"];
        assert!(region.is_null(), "region should be null when span is zero");
    }

    #[test]
    fn test_generate_sarif_report_with_span() {
        let findings = vec![LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        }];

        let report = generate_sarif_report(&findings);
        let value: serde_json::Value = serde_json::from_str(&report).unwrap();
        let region = &value["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 10);
        assert_eq!(region["startColumn"], 5);
        assert_eq!(region["endLine"], 12);
        assert_eq!(region["endColumn"], 15);
    }

    #[test]
    fn test_generate_sarif_report_schema_url() {
        let report = generate_sarif_report(&[]);
        let value: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert!(value["$schema"].as_str().unwrap().contains("sarif-2.1.0"));
    }

    #[test]
    fn test_generate_sarif_report_tool_name() {
        let report = generate_sarif_report(&[]);
        let value: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert_eq!(
            value["runs"][0]["tool"]["driver"]["name"],
            "cargo-cost-lint"
        );
    }

    fn sarif_finding(name: &str) -> LintFinding {
        LintFinding {
            name: name.to_string(),
            level: "warning".to_string(),
            file: "src/lib.rs".to_string(),
            span: Span {
                line_start: 1,
                line_end: 1,
                column_start: 1,
                column_end: 2,
            },
            message: format!("{} message", name),
            help: None,
            suggestion: None,
        }
    }

    #[test]
    fn test_generate_sarif_report_rules_are_deterministically_ordered() {
        // Issue #876: `tool.driver.rules` must be emitted in a deterministic
        // order for the same findings, independent of process hash seeding.
        let findings = vec![
            sarif_finding("z_lint"),
            sarif_finding("a_lint"),
            sarif_finding("m_lint"),
            sarif_finding("z_lint"),
        ];

        let report = generate_sarif_report(&findings);
        let value: serde_json::Value = serde_json::from_str(&report).unwrap();
        let ids: Vec<&str> = value["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].as_str().unwrap())
            .collect();
        // Deduplicated (one rule per distinct name) and sorted ascending.
        assert_eq!(ids, vec!["a_lint", "m_lint", "z_lint"]);
    }

    #[test]
    fn test_generate_sarif_report_is_byte_identical_across_renders() {
        // Issue #876 regression guard: rendering the same findings twice must
        // produce byte-identical SARIF, so unordered iteration can never come
        // back unnoticed.
        let findings = vec![
            sarif_finding("z_lint"),
            sarif_finding("a_lint"),
            sarif_finding("m_lint"),
        ];

        let first = generate_sarif_report(&findings);
        let second = generate_sarif_report(&findings);
        assert_eq!(first, second);
    }

    #[test]
    fn test_handle_finding_json_format() {
        let cli = crate::Cli {
            config: None,
            list_lints: false,
            explain: None,
            format: OutputFormat::Json,
            quiet: false,
            verbose: false,
            allow: vec![],
            warn: vec![],
            deny: vec![],
            package: vec![],
            workspace: false,
            no_cache: false,
            clear_cache: false,
            color: crate::ColorChoice::Auto,
            diff_only: false,
        };
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 1,
                line_end: 1,
                column_start: 1,
                column_end: 5,
            },
            message: "test".to_string(),
            help: None,
            suggestion: None,
        };
        let mut acc = Vec::new();
        let mut buf = Vec::new();
        handle_finding(&cli, &finding, &mut acc, &mut buf).unwrap();
        assert_eq!(acc.len(), 1);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("\"name\":\"test_lint\""));
    }

    #[test]
    fn test_handle_finding_github_format() {
        let cli = crate::Cli {
            config: None,
            list_lints: false,
            explain: None,
            format: OutputFormat::Github,
            quiet: false,
            verbose: false,
            allow: vec![],
            warn: vec![],
            deny: vec![],
            package: vec![],
            workspace: false,
            no_cache: false,
            clear_cache: false,
            color: crate::ColorChoice::Auto,
            diff_only: false,
        };
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 1,
                line_end: 1,
                column_start: 1,
                column_end: 5,
            },
            message: "test".to_string(),
            help: None,
            suggestion: None,
        };
        let mut acc = Vec::new();
        let mut buf = Vec::new();
        handle_finding(&cli, &finding, &mut acc, &mut buf).unwrap();
        assert_eq!(acc.len(), 1);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("::error file="));
    }

    #[test]
    fn test_handle_finding_text_format() {
        let cli = crate::Cli {
            config: None,
            list_lints: false,
            explain: None,
            format: OutputFormat::Text,
            quiet: false,
            verbose: false,
            allow: vec![],
            warn: vec![],
            deny: vec![],
            package: vec![],
            workspace: false,
            no_cache: false,
            clear_cache: false,
            color: crate::ColorChoice::Auto,
            diff_only: false,
        };
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 1,
                line_end: 1,
                column_start: 1,
                column_end: 5,
            },
            message: "test".to_string(),
            help: None,
            suggestion: None,
        };
        let mut acc = Vec::new();
        let mut buf = Vec::new();
        handle_finding(&cli, &finding, &mut acc, &mut buf).unwrap();
        assert_eq!(acc.len(), 1);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("error: [test_lint]"));
    }

    #[test]
    fn test_handle_finding_sarif_format() {
        let cli = crate::Cli {
            config: None,
            list_lints: false,
            explain: None,
            format: OutputFormat::Sarif,
            quiet: false,
            verbose: false,
            allow: vec![],
            warn: vec![],
            deny: vec![],
            package: vec![],
            workspace: false,
            no_cache: false,
            clear_cache: false,
            color: crate::ColorChoice::Auto,
            diff_only: false,
        };
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 1,
                line_end: 1,
                column_start: 1,
                column_end: 5,
            },
            message: "test".to_string(),
            help: None,
            suggestion: None,
        };
        let mut acc = Vec::new();
        let mut buf = Vec::new();
        handle_finding(&cli, &finding, &mut acc, &mut buf).unwrap();
        assert_eq!(acc.len(), 1);
        // SARIF format should not write to the writer
        assert!(buf.is_empty());
    }

    #[test]
    fn test_handle_finding_accumulates_multiple() {
        let cli = crate::Cli {
            config: None,
            list_lints: false,
            explain: None,
            format: OutputFormat::Sarif,
            quiet: false,
            verbose: false,
            allow: vec![],
            warn: vec![],
            deny: vec![],
            package: vec![],
            workspace: false,
            no_cache: false,
            clear_cache: false,
            color: crate::ColorChoice::Auto,
            diff_only: false,
        };
        let finding1 = LintFinding {
            name: "lint_a".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 1,
                line_end: 1,
                column_start: 1,
                column_end: 5,
            },
            message: "test1".to_string(),
            help: None,
            suggestion: None,
        };
        let finding2 = LintFinding {
            name: "lint_b".to_string(),
            level: "warning".to_string(),
            file: "src/lib.rs".to_string(),
            span: Span {
                line_start: 2,
                line_end: 2,
                column_start: 1,
                column_end: 5,
            },
            message: "test2".to_string(),
            help: None,
            suggestion: None,
        };
        let mut acc = Vec::new();
        let mut buf = Vec::new();
        handle_finding(&cli, &finding1, &mut acc, &mut buf).unwrap();
        handle_finding(&cli, &finding2, &mut acc, &mut buf).unwrap();
        assert_eq!(acc.len(), 2);
        assert_eq!(acc[0].name, "lint_a");
        assert_eq!(acc[1].name, "lint_b");
    }

    #[test]
    fn test_lint_finding_serialization() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: Some("help text".to_string()),
            suggestion: Some("suggestion text".to_string()),
        };
        let json = serde_json::to_string(&finding).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["name"], "test_lint");
        assert_eq!(value["level"], "error");
        assert_eq!(value["file"], "src/main.rs");
        assert_eq!(value["span"]["line_start"], 10);
        assert_eq!(value["span"]["line_end"], 12);
        assert_eq!(value["span"]["column_start"], 5);
        assert_eq!(value["span"]["column_end"], 15);
        assert_eq!(value["message"], "test message");
        assert_eq!(value["help"], "help text");
        assert_eq!(value["suggestion"], "suggestion text");
    }

    #[test]
    fn test_lint_finding_serialization_skips_none_fields() {
        let finding = LintFinding {
            name: "test_lint".to_string(),
            level: "error".to_string(),
            file: "src/main.rs".to_string(),
            span: Span {
                line_start: 10,
                line_end: 12,
                column_start: 5,
                column_end: 15,
            },
            message: "test message".to_string(),
            help: None,
            suggestion: None,
        };
        let json = serde_json::to_string(&finding).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(
            value.get("help").is_none(),
            "help should be skipped when None"
        );
        assert!(
            value.get("suggestion").is_none(),
            "suggestion should be skipped when None"
        );
    }

    #[test]
    fn test_output_format_value_enum() {
        // Verify all variants exist and are distinct
        let formats = [
            OutputFormat::Text,
            OutputFormat::Json,
            OutputFormat::Sarif,
            OutputFormat::Github,
        ];
        for (i, f1) in formats.iter().enumerate() {
            for (j, f2) in formats.iter().enumerate() {
                if i == j {
                    assert_eq!(f1, f2);
                } else {
                    assert_ne!(f1, f2);
                }
            }
        }
    }

    #[test]
    fn test_span_serialization() {
        let span = Span {
            line_start: 1,
            line_end: 2,
            column_start: 3,
            column_end: 4,
        };
        let json = serde_json::to_string(&span).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["line_start"], 1);
        assert_eq!(value["line_end"], 2);
        assert_eq!(value["column_start"], 3);
        assert_eq!(value["column_end"], 4);
    }

    #[test]
    fn test_sarif_report_serialization() {
        let report = SarifReport {
            schema: "test".to_string(),
            version: "2.1.0".to_string(),
            runs: vec![],
        };
        let json = serde_json::to_string(&report).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["$schema"], "test");
        assert_eq!(value["version"], "2.1.0");
    }

    #[test]
    fn test_sarif_tool_driver_serialization() {
        let driver = SarifToolDriver {
            name: "test".to_string(),
            version: "1.0".to_string(),
            information_uri: Some("https://example.com".to_string()),
            rules: vec![],
        };
        let json = serde_json::to_string(&driver).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["name"], "test");
        assert_eq!(value["version"], "1.0");
        assert_eq!(value["informationUri"], "https://example.com");
    }

    #[test]
    fn test_sarif_tool_driver_skips_empty_rules() {
        let driver = SarifToolDriver {
            name: "test".to_string(),
            version: "1.0".to_string(),
            information_uri: None,
            rules: vec![],
        };
        let json = serde_json::to_string(&driver).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value.get("rules").is_none());
        assert!(value.get("informationUri").is_none());
    }

    #[test]
    fn test_sarif_region_serialization() {
        let region = SarifRegion {
            start_line: 10,
            start_column: Some(5),
            end_line: Some(12),
            end_column: Some(15),
        };
        let json = serde_json::to_string(&region).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["startLine"], 10);
        assert_eq!(value["startColumn"], 5);
        assert_eq!(value["endLine"], 12);
        assert_eq!(value["endColumn"], 15);
    }

    #[test]
    fn test_sarif_region_skips_none_fields() {
        let region = SarifRegion {
            start_line: 10,
            start_column: None,
            end_line: None,
            end_column: None,
        };
        let json = serde_json::to_string(&region).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["startLine"], 10);
        assert!(value.get("startColumn").is_none());
        assert!(value.get("endLine").is_none());
        assert!(value.get("endColumn").is_none());
    }

    #[test]
    fn test_sarif_result_serialization() {
        let result = SarifResult {
            rule_id: "test_lint".to_string(),
            level: "error".to_string(),
            message: SarifMessage {
                text: "test message".to_string(),
            },
            locations: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["ruleId"], "test_lint");
        assert_eq!(value["level"], "error");
        assert_eq!(value["message"]["text"], "test message");
    }

    #[test]
    fn test_sarif_rule_serialization() {
        let rule = SarifRule {
            id: "test_lint".to_string(),
            short_description: SarifRuleShortDescription {
                text: "Test lint".to_string(),
            },
        };
        let json = serde_json::to_string(&rule).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["id"], "test_lint");
        assert_eq!(value["shortDescription"]["text"], "Test lint");
    }

    #[test]
    fn test_sarif_location_serialization() {
        let location = SarifLocation {
            physical_location: SarifPhysicalLocation {
                artifact_location: SarifArtifactLocation {
                    uri: "file:///test.rs".to_string(),
                },
                region: None,
            },
        };
        let json = serde_json::to_string(&location).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            value["physicalLocation"]["artifactLocation"]["uri"],
            "file:///test.rs"
        );
        assert!(value["physicalLocation"].get("region").is_none());
    }

    #[test]
    fn test_sarif_physical_location_with_region() {
        let location = SarifPhysicalLocation {
            artifact_location: SarifArtifactLocation {
                uri: "file:///test.rs".to_string(),
            },
            region: Some(SarifRegion {
                start_line: 1,
                start_column: Some(1),
                end_line: Some(2),
                end_column: Some(2),
            }),
        };
        let json = serde_json::to_string(&location).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["region"]["startLine"], 1);
    }

    #[test]
    fn test_sarif_run_serialization() {
        let run = SarifRun {
            tool: SarifTool {
                driver: SarifToolDriver {
                    name: "test".to_string(),
                    version: "1.0".to_string(),
                    information_uri: None,
                    rules: vec![],
                },
            },
            results: vec![],
        };
        let json = serde_json::to_string(&run).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["tool"]["driver"]["name"], "test");
        assert!(value["results"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_sarif_message_serialization() {
        let msg = SarifMessage {
            text: "test".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["text"], "test");
    }

    #[test]
    fn test_sarif_artifact_location_serialization() {
        let loc = SarifArtifactLocation {
            uri: "file:///test.rs".to_string(),
        };
        let json = serde_json::to_string(&loc).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["uri"], "file:///test.rs");
    }

    #[test]
    fn test_sarif_rule_short_description_serialization() {
        let desc = SarifRuleShortDescription {
            text: "Test".to_string(),
        };
        let json = serde_json::to_string(&desc).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["text"], "Test");
    }

    #[test]
    fn test_sarif_tool_serialization() {
        let tool = SarifTool {
            driver: SarifToolDriver {
                name: "test".to_string(),
                version: "1.0".to_string(),
                information_uri: None,
                rules: vec![],
            },
        };
        let json = serde_json::to_string(&tool).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["driver"]["name"], "test");
    }
}
