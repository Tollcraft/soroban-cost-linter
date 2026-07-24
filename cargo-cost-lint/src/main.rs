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

include!(concat!(env!("OUT_DIR"), "/lint_names.rs"));

fn validate_and_build_flags(config: &BudgetConfig) -> Result<Vec<String>, String> {
    let mut lint_flags = Vec::new();

    if let Some(lints) = &config.lints {
        for (lint, level) in lints {
            if !LINT_NAMES.contains(&lint.as_str()) {
                let valid = LINT_NAMES.join(", ");
                return Err(format!(
                    "Error: Unknown lint name '{}' in budget.toml. Valid lints are: {}",
                    lint, valid
                ));
            }

            let level_flag = match level.as_str() {
                "allow" => "-A",
                "warn" => "-W",
                "deny" => "-D",
                _ => {
                    return Err(format!(
                        "Error: Unknown lint level '{}' for lint '{}'",
                        level, lint
                    ))
                }
            };

            lint_flags.push(format!("{} {}", level_flag, lint));
        }
    }

    Ok(lint_flags)
}

fn main() {
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
        match fs::read_to_string(&cli.config) {
            Ok(config_str) => match toml::from_str::<BudgetConfig>(&config_str) {
                Ok(config) => match validate_and_build_flags(&config) {
                    Ok(flags) => lint_flags = flags,
                    Err(e) => {
                        eprintln!("{}", e);
                        exit(1);
                    }
                },
                Err(_) => eprintln!("Warning: Failed to parse {}", cli.config),
            },
            Err(_) => eprintln!("Warning: Failed to read {}", cli.config),
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
        let mut rustflags = std::env::var("DYLINT_RUSTFLAGS").unwrap_or_default();

        for flag in lint_flags {
            if !rustflags.is_empty() {
                rustflags.push(' ');
            }
            rustflags.push_str(&flag);
        }

        cmd.env("DYLINT_RUSTFLAGS", rustflags);
    }

    match cmd.status() {
        Ok(status) => {
            if !status.success() {
                exit(status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!("Failed to execute cargo dylint: {}", e);
            exit(1);
        }
    }
}
