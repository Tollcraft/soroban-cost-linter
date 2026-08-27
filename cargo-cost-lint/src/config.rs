use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::error::{LinterError, LinterResult};

#[derive(Deserialize, Debug, Default, Clone)]
// The newer config generation (fallback defaults, issue #211/#13). Not yet
// wired into `main()`, which still uses the older validated path -- see the
// note on `parse_budget_config` in main.rs.
// Kept: scaffolding for newer config generation
#[allow(dead_code)]
pub struct Config {
    lints: Option<HashMap<String, String>>,
}

// Kept: part of the newer config generation
#[allow(dead_code)]
impl Config {
    /// Reads and parses the budget.toml at `path`.
    ///
    /// Returns the default (empty) config when the file is absent, but
    /// propagates parse errors so malformed files are surfaced to the user
    /// rather than silently ignored.
    pub fn from_file_or_default(path: &Path) -> LinterResult<Self> {
        if !path.exists() {
            return Ok(Config::default());
        }
        let content = fs::read_to_string(path)?;
        let config = toml::from_str::<Config>(&content).map_err(|e| {
            LinterError::Other(format!("failed to parse {}: {}", path.display(), e))
        })?;
        Ok(config)
    }

    /// Converts the lint severity settings into rustc-compatible flags for
    /// `DYLINT_RUSTFLAGS`.
    ///
    /// Each severity maps to a flag prefix:
    ///   "allow"  → `-A`     (allow the lint at module level)
    ///   "warn"   → `-W`     (upgrade to warning)
    ///   "deny"   → `-D`     (upgrade to error)
    ///   "forbid" → `-F`     (forbid at module level)
    ///
    /// Unknown severity values are skipped with a warning to stderr.
    pub fn to_lint_flags(&self) -> Vec<String> {
        let Some(lints) = &self.lints else {
            return Vec::new();
        };

        let mut flags = Vec::new();
        for (lint_name, severity) in lints {
            let prefix = match severity.as_str() {
                "allow" => "-A",
                "warn" => "-W",
                "deny" => "-D",
                "forbid" => "-F",
                other => {
                    eprintln!(
                        "warning: unknown severity '{}' for lint '{}' in budget.toml \
                         (expected allow, warn, deny, or forbid) — skipping",
                        other, lint_name
                    );
                    continue;
                }
            };
            // Normalize to lowercase: rustc lint names are case-sensitive and
            // the officially declared names are always lowercase.
            flags.push(format!("{} {}", prefix, lint_name.to_lowercase()));
        }
        flags
    }
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct BudgetConfig {
    pub lints: Option<HashMap<String, String>>,
}

impl BudgetConfig {
    /// Reads and parses `path` into a `BudgetConfig`, validating that every
    /// configured lint name is one of `known_lints` and every level is one
    /// of `allow`/`warn`/`deny`. This is the single canonical entry point
    /// for loading a `budget.toml` — the file may be read from anywhere on
    /// disk, but always goes through this same validation.
    pub fn from_file_validated(path: &Path, known_lints: &[&str]) -> Result<Self, String> {
        let path_display = path.display();
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Error: Failed to read {}: {}", path_display, e))?;
        let config: BudgetConfig = toml::from_str(&content)
            .map_err(|e| format!("Error: Failed to parse {}: {}", path_display, e))?;

        if let Some(lints) = &config.lints {
            for (lint, level) in lints {
                if !known_lints.contains(&lint.as_str()) {
                    return Err(format!(
                        "Error: Unknown lint name '{}' in {}. Valid lints: {}",
                        lint,
                        path_display,
                        known_lints.join(", ")
                    ));
                }
                if !matches!(level.as_str(), "allow" | "warn" | "deny") {
                    return Err(format!(
                        "Error: Unknown lint level '{}' for '{}' in {}. Valid levels are allow, warn, and deny.",
                        level, lint, path_display
                    ));
                }
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;

    use tempfile::tempdir;

    use super::*;
    const KNOWN_LINTS: &[&str] = &["soroban_storage_in_loop", "redundant_env_clone"];

    fn write_file(path: &Path, contents: &str) {
        let mut f = File::create(path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
    }

    #[test]
    fn default_config_has_no_lints() {
        let config = BudgetConfig::default();
        assert!(config.lints.is_none());
    }

    #[test]
    fn from_file_validated_returns_error_for_missing_file() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("nonexistent.toml");
        let config = Config::from_file_or_default(&missing).unwrap();
        assert!(config.lints.is_none());
        let result = BudgetConfig::from_file_validated(&missing, KNOWN_LINTS);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to read"));
    }

    #[test]
    fn from_file_validated_returns_error_for_malformed_toml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(&path, "this is not valid toml [[[");
        let result = BudgetConfig::from_file_validated(&path, KNOWN_LINTS);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse"));
    }

    #[test]
    fn from_file_validated_parses_valid_toml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(
            &path,
            r#"[lints]
soroban_storage_in_loop = "deny"
"#,
        );
        let _config = Config::from_file_or_default(&path).unwrap();
        let config = BudgetConfig::from_file_validated(&path, KNOWN_LINTS).unwrap();
        let lints = config.lints.expect("lints should be present");
        assert_eq!(
            lints.get("soroban_storage_in_loop").map(|s| s.as_str()),
            Some("deny")
        );
    }

    #[test]
    fn from_file_validated_handles_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(&path, "this is not valid toml [[[");
        let result = Config::from_file_or_default(&path);
        assert!(result.is_err());
        write_file(&path, "");
        let config = BudgetConfig::from_file_validated(&path, KNOWN_LINTS).unwrap();
        assert!(config.lints.is_none());
    }

    #[test]
    fn from_file_validated_rejects_unknown_lint_name() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(&path, "");
        let config = Config::from_file_or_default(&path).unwrap();
        assert!(config.lints.is_none());
        write_file(&path, "[lints]\nnot_a_real_lint = \"deny\"\n");
        let result = BudgetConfig::from_file_validated(&path, KNOWN_LINTS);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown lint name"));
    }

    #[test]
    fn from_file_validated_rejects_unknown_level() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(&path, "[lints]\nsoroban_storage_in_loop = \"oops\"\n");
        let result = BudgetConfig::from_file_validated(&path, KNOWN_LINTS);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown lint level"));
    }

    #[test]
    fn to_lint_flags_empty_when_no_lints() {
        let config = Config::default();
        assert!(config.to_lint_flags().is_empty());
    }

    #[test]
    fn to_lint_flags_maps_severity_to_rustc_flag() {
        let mut lints = HashMap::new();
        lints.insert("soroban_storage_in_loop".to_string(), "deny".to_string());
        lints.insert("redundant_env_clone".to_string(), "warn".to_string());
        lints.insert(
            "unnecessary_host_function_call".to_string(),
            "allow".to_string(),
        );
        lints.insert("host_in_loop".to_string(), "forbid".to_string());
        let config = Config { lints: Some(lints) };
        let flags = config.to_lint_flags();
        assert_eq!(flags.len(), 4);
        assert!(flags.contains(&"-D soroban_storage_in_loop".to_string()));
        assert!(flags.contains(&"-W redundant_env_clone".to_string()));
        assert!(flags.contains(&"-A unnecessary_host_function_call".to_string()));
        assert!(flags.contains(&"-F host_in_loop".to_string()));
    }

    #[test]
    fn to_lint_flags_skips_unknown_severity() {
        let mut lints = HashMap::new();
        lints.insert("some_lint".to_string(), "bogus".to_string());
        lints.insert("valid_lint".to_string(), "deny".to_string());
        let config = Config { lints: Some(lints) };
        let flags = config.to_lint_flags();
        assert_eq!(flags.len(), 1);
        assert!(flags.contains(&"-D valid_lint".to_string()));
    }

    #[test]
    fn from_file_validated_ignores_unknown_sections() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(
            &path,
            r#"network = "testnet"
source = "alice"

[margin]
cpu_margin    = 1.50
memory_margin = 1.25
read_margin   = 2.00
write_margin  = 3.00

[scenarios.full_workflow]
package = "amm-pool-contract"
functions = ["deposit", "swap", "withdraw"]

[functions.do_expensive_work]
args = ["--n", "10000"]
cpu_limit = 5000000
read_limit = 5000
write_limit = 1000

[lints]
soroban_storage_in_loop = "deny"
"#,
        );
        let config = BudgetConfig::from_file_validated(&path, KNOWN_LINTS).unwrap();
        let lints = config.lints.expect("lints should be present");
        assert_eq!(
            lints.get("soroban_storage_in_loop").map(|s| s.as_str()),
            Some("deny")
        );
    }

    #[test]
    fn from_file_validated_works_without_lints_section() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(
            &path,
            r#"network = "testnet"
source = "alice"

[margin]
cpu_margin    = 1.50
memory_margin = 1.25
read_margin   = 2.00
write_margin  = 3.00

[scenarios.full_workflow]
package = "amm-pool-contract"
functions = ["deposit", "swap", "withdraw"]
"#,
        );
        let config = BudgetConfig::from_file_validated(&path, KNOWN_LINTS).unwrap();
        assert!(config.lints.is_none());
    }

    #[test]
    fn from_file_validated_does_not_parse_obsolete_budget_wrapper_table() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(
            &path,
            r#"[budget]
lints = { "soroban_storage_in_loop" = "deny" }
"#,
        );
        let config = BudgetConfig::from_file_validated(&path, KNOWN_LINTS).unwrap();
        assert!(
            config.lints.is_none(),
            "obsolete [budget] wrapper schema should not be parsed as top-level [lints]"
        );

        let cfg = Config::from_file_or_default(&path).unwrap();
        assert!(
            cfg.to_lint_flags().is_empty(),
            "obsolete [budget] wrapper schema should produce no lint flags"
        );
    }
}
