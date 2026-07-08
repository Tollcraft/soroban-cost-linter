use clap::Parser;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::process::{exit, Command};

#[derive(Parser, Debug)]
#[command(name = "cargo-cost-lint")]
#[command(about = "CLI wrapper for soroban-cost-linter")]
struct Cli {
    #[arg(long, help = "Path to budget.toml", default_value = "budget.toml")]
    config: String,
}

#[derive(Deserialize, Debug)]
struct BudgetConfig {
    lints: Option<std::collections::HashMap<String, String>>,
}

fn main() {
    // Skip the first arg if it is "cost-lint" (when invoked as a cargo subcommand)
    let mut args = std::env::args().collect::<Vec<_>>();
    if args.len() > 1 && args[1] == "cost-lint" {
        args.remove(1);
    }

    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => {
            e.print().unwrap();
            exit(1);
        }
    };

    let mut lint_flags = Vec::new();

    if Path::new(&cli.config).exists() {
        if let Ok(config_str) = fs::read_to_string(&cli.config) {
            if let Ok(config) = toml::from_str::<BudgetConfig>(&config_str) {
                if let Some(lints) = config.lints {
                    for (lint, level) in lints {
                        let level_flag = match level.as_str() {
                            "allow" => "-A",
                            "warn" => "-W",
                            "deny" => "-D",
                            _ => {
                                eprintln!("Unknown lint level: {}", level);
                                continue;
                            }
                        };
                        lint_flags.push(format!("{}={}", level_flag, lint));
                    }
                }
            } else {
                eprintln!("Warning: Failed to parse {}", cli.config);
            }
        }
    } else {
        eprintln!(
            "Warning: {} not found, using default lint levels.",
            cli.config
        );
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("dylint");
    cmd.arg("--lib");
    cmd.arg("soroban_cost_lints");

    if !lint_flags.is_empty() {
        cmd.arg("--");
        for flag in lint_flags {
            let parts: Vec<&str> = flag.split('=').collect();
            cmd.arg(parts[0]);
            // Add the soroban_cost prefix if not present or just the lint name?
            // Dylint uses the exact lint name declared in declare_lint!
            cmd.arg(parts[1]);
        }
    }

    let status = cmd
        .status()
        .expect("Failed to execute cargo dylint. Is cargo-dylint installed?");
    if !status.success() {
        exit(status.code().unwrap_or(1));
    }
}
