use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize, Debug, Default, Clone, PartialEq, Eq)]
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
    const KNOWN_LINTS: &[&str] = &[
        "soroban_storage_in_loop",
        "redundant_env_clone",
        "redundant_address_clone",
    ];

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
        let config = BudgetConfig::from_file_validated(&path, KNOWN_LINTS).unwrap();
        let lints = config.lints.as_ref().expect("lints should be present");
        assert_eq!(
            lints.get("soroban_storage_in_loop").map(|s| s.as_str()),
            Some("deny")
        );
    }

    #[test]
    fn from_file_validated_handles_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(&path, "");
        let config = BudgetConfig::from_file_validated(&path, KNOWN_LINTS).unwrap();
        assert!(config.lints.is_none());
    }

    #[test]
    fn from_file_validated_rejects_unknown_lint_name() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
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
    }

    #[test]
    fn from_file_validated_rejects_forbid_level() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(&path, "[lints]\nsoroban_storage_in_loop = \"forbid\"\n");
        let result = BudgetConfig::from_file_validated(&path, KNOWN_LINTS);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Unknown lint level 'forbid'"), "{err}");
        assert!(err.contains("allow, warn, and deny"), "{err}");
    }
}
