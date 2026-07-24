use clap::Parser;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

#[derive(Parser, Debug)]
#[command(name = "cargo-cost-lint")]
#[command(about = "CLI wrapper for soroban-cost-linter")]
struct Cli {
    #[arg(long, help = "Path to budget.toml")]
    config: Option<String>,
}

#[derive(Deserialize, Debug)]
struct BudgetConfig {
    lints: Option<std::collections::HashMap<String, String>>,
}

fn find_workspace_root_from(start: &Path) -> Option<PathBuf> {
    let mut dir = start.canonicalize().ok()?;
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.is_file() {
            if let Ok(content) = fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return Some(dir);
                }
            }
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

fn find_workspace_root() -> Option<PathBuf> {
    find_workspace_root_from(&std::env::current_dir().ok()?)
}

fn resolve_config(config_arg: Option<&str>) -> Option<PathBuf> {
    if let Some(path) = config_arg {
        let p = Path::new(path);
        if p.exists() {
            Some(p.to_path_buf())
        } else {
            None
        }
    } else {
        find_workspace_root()
            .map(|root| root.join("budget.toml"))
            .filter(|p| p.exists())
    }
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

    if let Some(ref path) = resolve_config(cli.config.as_deref()) {
        eprintln!("Using config: {}", path.display());
        if let Ok(config_str) = fs::read_to_string(path) {
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
                        lint_flags.push(format!("{} {}", level_flag, lint));
                    }
                }
            } else {
                eprintln!("Warning: Failed to parse {}", path.display());
            }
        }
    } else {
        eprintln!(
            "Warning: budget.toml not found, using default lint levels."
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

    let status = cmd
        .status()
        .expect("Failed to execute cargo dylint. Is cargo-dylint installed?");
    if !status.success() {
        exit(status.code().unwrap_or(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolve_explicit_path() {
        let dir = tempfile::tempdir().unwrap();
        let budget = dir.path().join("my-budget.toml");
        fs::write(&budget, "[lints]\nfoo = \"deny\"\n").unwrap();

        let found = resolve_config(Some(budget.to_str().unwrap()));
        assert!(found.is_some());
        assert_eq!(found.unwrap().canonicalize().unwrap(), budget.canonicalize().unwrap());
    }

    #[test]
    fn resolve_explicit_path_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let budget = dir.path().join("nonexistent.toml");
        assert!(resolve_config(Some(budget.to_str().unwrap())).is_none());
    }

    #[test]
    fn resolve_no_explicit_path_finds_workspace_budget() {
        let dir = tempfile::tempdir().unwrap();
        let ws_root = dir.path().join("ws");
        fs::create_dir_all(&ws_root).unwrap();
        fs::write(
            ws_root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"member\"]\n",
        )
        .unwrap();
        let budget = ws_root.join("budget.toml");
        fs::write(&budget, "[lints]\nfoo = \"deny\"\n").unwrap();

        let found = resolve_config_from(&ws_root, None);
        assert!(found.is_some());
        assert_eq!(found.unwrap().canonicalize().unwrap(), budget.canonicalize().unwrap());
    }

    #[test]
    fn find_workspace_root_from_workspace_root() {
        let dir = tempfile::tempdir().unwrap();
        let ws_root = dir.path().join("ws");
        fs::create_dir_all(&ws_root).unwrap();
        fs::write(
            ws_root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"member\"]\n",
        )
        .unwrap();

        let found = find_workspace_root_from(&ws_root);
        assert!(found.is_some());
        assert_eq!(found.unwrap().canonicalize().unwrap(), ws_root.canonicalize().unwrap());
    }

    #[test]
    fn find_workspace_root_from_member_dir() {
        let dir = tempfile::tempdir().unwrap();
        let ws_root = dir.path().join("ws");
        let member = ws_root.join("member");
        fs::create_dir_all(&member).unwrap();
        fs::write(
            ws_root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"member\"]\n",
        )
        .unwrap();
        fs::write(
            member.join("Cargo.toml"),
            "[package]\nname = \"member\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();

        let found = find_workspace_root_from(&member);
        assert!(found.is_some());
        assert_eq!(found.unwrap().canonicalize().unwrap(), ws_root.canonicalize().unwrap());
    }

    #[test]
    fn find_workspace_root_no_workspace() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"foo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();

        assert!(find_workspace_root_from(dir.path()).is_none());
    }

    #[test]
    fn resolve_config_prefers_explicit_over_workspace() {
        let dir = tempfile::tempdir().unwrap();
        // Create workspace with budget.toml
        let ws_root = dir.path().join("ws");
        fs::create_dir_all(&ws_root).unwrap();
        fs::write(
            ws_root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"member\"]\n",
        )
        .unwrap();
        let ws_budget = ws_root.join("budget.toml");
        fs::write(&ws_budget, "[lints]\nfoo = \"deny\"\n").unwrap();

        // Create an explicit budget.toml elsewhere
        let explicit_budget = dir.path().join("explicit.toml");
        fs::write(&explicit_budget, "[lints]\nbar = \"allow\"\n").unwrap();

        let found = resolve_config_from(&ws_root, Some(explicit_budget.to_str().unwrap()));
        assert!(found.is_some());
        assert_eq!(found.unwrap().canonicalize().unwrap(), explicit_budget.canonicalize().unwrap());
    }

    fn resolve_config_from(start: &Path, config_arg: Option<&str>) -> Option<PathBuf> {
        if let Some(path) = config_arg {
            let p = Path::new(path);
            if p.exists() {
                return Some(p.to_path_buf());
            }
            return None;
        }
        find_workspace_root_from(start)
            .map(|root| root.join("budget.toml"))
            .filter(|p| p.exists())
    }
}
