use clap::{Parser, ValueEnum};
use serde::Serialize;
use std::fmt;
use tabled::Tabled;

#[derive(Tabled)]
struct ReportRow {
    package: String,
    function: String,
    metric: String,
    value: String,
    share_of_limit: String,
    flag: String,
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Parser, Debug)]
#[command(name = "cargo budget-report")]
#[command(about = "Reports Soroban function resource costs as a share of network resource limits.")]
struct Cli {
    #[arg(long, help = "Soroban network RPC URL (default: Soroban testnet)")]
    network: Option<String>,

    #[arg(
        long,
        default_value = "50.0",
        help = "Threshold percentage above which a function is flagged"
    )]
    threshold: f64,

    #[arg(long, value_enum, default_value_t = OutputFormat::Text, help = "Output format")]
    format: OutputFormat,
}

#[derive(Debug, Clone)]
struct NetworkLimits {
    max_instructions: u64,
    max_read_bytes: u64,
    max_write_bytes: u64,
    protocol_version: u32,
}

#[derive(Debug, Clone)]
enum LimitSource {
    FetchedFromNetwork {
        network_url: String,
    },
    HardcodedProtocolVersion {
        protocol_version: u32,
        documented_in: String,
    },
}

impl fmt::Display for LimitSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LimitSource::FetchedFromNetwork { network_url } => {
                write!(f, "fetched from network ({network_url})")
            }
            LimitSource::HardcodedProtocolVersion {
                protocol_version,
                documented_in,
            } => {
                write!(
                    f,
                    "hardcoded for Protocol {protocol_version} ({documented_in}; these limits change with protocol upgrades, review when upgrading)"
                )
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct CostReport {
    package: String,
    function: String,
    metric: String,
    value: u64,
    limit: u64,
    share_of_limit_pct: f64,
    flagged: bool,
}

impl CostReport {
    fn new(
        package: impl Into<String>,
        function: impl Into<String>,
        metric: impl Into<String>,
        value: u64,
        limit: u64,
        threshold_pct: f64,
    ) -> Self {
        let share_pct = if limit > 0 {
            (value as f64 / limit as f64) * 100.0
        } else {
            0.0
        };
        let share_pct = (share_pct * 100.0).round() / 100.0;
        let flagged = share_pct > threshold_pct;
        CostReport {
            package: package.into(),
            function: function.into(),
            metric: metric.into(),
            value,
            limit,
            share_of_limit_pct: share_pct,
            flagged,
        }
    }
}

fn default_limits() -> NetworkLimits {
    NetworkLimits {
        max_instructions: 10_000_000_000,
        max_read_bytes: 100_000_000,
        max_write_bytes: 100_000_000,
        protocol_version: 22,
    }
}

fn fetch_network_limits(network_url: &str) -> anyhow::Result<(NetworkLimits, LimitSource)> {
    let rpc_url = if network_url.ends_with('/') {
        format!("{network_url}soroban/v1/ledger/core")
    } else {
        format!("{network_url}/soroban/v1/ledger/core")
    };

    let response = ureq::get(&rpc_url).call().map_err(|e| {
        anyhow::anyhow!(
            "Failed to fetch network limits from {rpc_url}: {e}. \
                 Using protocol-versioned hardcoded fallback limits."
        )
    })?;

    let body = response.into_string()?;
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| anyhow::anyhow!("Failed to parse network response: {e}"))?;

    let proto_version = json
        .get("protocol_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(22) as u32;

    let limits = NetworkLimits {
        max_instructions: json
            .get("max_instructions")
            .and_then(|v| v.as_u64())
            .unwrap_or(default_limits().max_instructions),
        max_read_bytes: json
            .get("max_read_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(default_limits().max_read_bytes),
        max_write_bytes: json
            .get("max_write_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(default_limits().max_write_bytes),
        protocol_version: proto_version,
    };

    Ok((
        limits,
        LimitSource::FetchedFromNetwork {
            network_url: rpc_url,
        },
    ))
}

fn get_limits(network: Option<&str>) -> anyhow::Result<(NetworkLimits, LimitSource)> {
    match network {
        Some(url) => fetch_network_limits(url),
        None => {
            let testnet_url = "https://soroban-testnet.stellar.org";
            match fetch_network_limits(testnet_url) {
                Ok(result) => Ok(result),
                Err(_) => {
                    let limits = default_limits();
                    Ok((
                        limits.clone(),
                        LimitSource::HardcodedProtocolVersion {
                            protocol_version: limits.protocol_version,
                            documented_in: String::from(
                                "soroban-budget-assert Soroban protocol config; \
                 Protocol 22 limits are 10B instructions, 100MB read/write.",
                            ),
                        },
                    ))
                }
            }
        }
    }
}

fn format_share(share_pct: f64) -> String {
    format!("{share_pct:.2}%")
}

fn render_text(reports: &[CostReport], limits: &NetworkLimits, threshold: f64) {
    let rows: Vec<ReportRow> = reports
        .iter()
        .map(|r| {
            let flag = if r.flagged { " \u{26a0}" } else { "" };
            ReportRow {
                package: r.package.clone(),
                function: r.function.clone(),
                metric: r.metric.clone(),
                value: format_value(r.value, &r.metric),
                share_of_limit: format_share(r.share_of_limit_pct),
                flag: flag.to_string(),
            }
        })
        .collect();

    let mut table = tabled::Table::new(rows);
    table.with(tabled::settings::Style::modern());

    let flagged: Vec<_> = reports.iter().filter(|r| r.flagged).collect();
    if !flagged.is_empty() {
        eprintln!(
            "\nWarning: {} function(s) exceed the {threshold:.1}% threshold:",
            flagged.len()
        );
        for r in &flagged {
            eprintln!(
                "  {}::{} {} = {} ({:.2}% of limit)",
                r.package, r.function, r.metric, r.value, r.share_of_limit_pct
            );
        }
    }

    println!("{table}");
    println!(
        "\nLimits used: Protocol {proto_version}, {max_inst} inst., {max_read} B read, {max_write} B write.",
        proto_version = limits.protocol_version,
        max_inst = limits.max_instructions,
        max_read = limits.max_read_bytes,
        max_write = limits.max_write_bytes,
    );
}

fn format_value(value: u64, metric: &str) -> String {
    if metric.contains("Byte") {
        format_value_bytes(value)
    } else {
        format_value_instructions(value)
    }
}

fn format_value_instructions(value: u64) -> String {
    if value >= 1_000_000_000 {
        format!("{:.1}B", value as f64 / 1_000_000_000.0)
    } else if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}K", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

fn format_value_bytes(value: u64) -> String {
    if value >= 1_000_000_000 {
        format!("{:.1}GB", value as f64 / 1_000_000_000.0)
    } else if value >= 1_000_000 {
        format!("{:.1}MB", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}KB", value as f64 / 1_000.0)
    } else {
        format!("{value} B")
    }
}

fn render_json(
    reports: &[CostReport],
    limits: &NetworkLimits,
    source: &LimitSource,
    threshold: f64,
) {
    let output = serde_json::json!({
        "limits": {
            "max_instructions": limits.max_instructions,
            "max_read_bytes": limits.max_read_bytes,
            "max_write_bytes": limits.max_write_bytes,
            "protocol_version": limits.protocol_version,
        },
        "limit_source": source.to_string(),
        "threshold_pct": threshold,
        "functions": reports,
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&output).expect("JSON serialization is infallible")
    );
}

struct SimulatedFunction {
    package: String,
    function: String,
    metrics: Vec<(String, u64)>,
}

fn simulate_functions() -> Vec<SimulatedFunction> {
    vec![
        SimulatedFunction {
            package: "amm-pool".to_string(),
            function: "do_expensive_work".to_string(),
            metrics: vec![
                ("CPU Instructions".to_string(), 901_816),
                ("Read Bytes".to_string(), 4_096),
                ("Write Bytes".to_string(), 1_024),
            ],
        },
        SimulatedFunction {
            package: "amm-pool".to_string(),
            function: "simple_transfer".to_string(),
            metrics: vec![
                ("CPU Instructions".to_string(), 50_000),
                ("Read Bytes".to_string(), 512),
                ("Write Bytes".to_string(), 256),
            ],
        },
    ]
}

fn limit_for_metric(metric: &str, limits: &NetworkLimits) -> u64 {
    if metric.contains("CPU") || metric.contains("Instruction") {
        limits.max_instructions
    } else if metric.contains("Read") {
        limits.max_read_bytes
    } else if metric.contains("Write") {
        limits.max_write_bytes
    } else {
        limits.max_instructions
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.threshold < 0.0 || cli.threshold > 100.0 {
        anyhow::bail!(
            "--threshold must be between 0 and 100, got {}",
            cli.threshold
        );
    }

    let (limits, source) = get_limits(cli.network.as_deref())?;

    let reports: Vec<CostReport> = simulate_functions()
        .iter()
        .flat_map(|func| {
            func.metrics.iter().map(|(metric, value)| {
                CostReport::new(
                    &func.package,
                    &func.function,
                    metric,
                    *value,
                    limit_for_metric(metric, &limits),
                    cli.threshold,
                )
            })
        })
        .collect();

    match cli.format {
        OutputFormat::Text => render_text(&reports, &limits, cli.threshold),
        OutputFormat::Json => render_json(&reports, &limits, &source, cli.threshold),
    }

    Ok(())
}
