use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::{LinterError, LinterResult};

/// Folds a user-written lint name into the form the lint inventory is keyed by.
///
/// `LINT_NAMES` is lower-cased as `build.rs` generates it, and the `-A`/`-W`/`-D`
/// flags handed to rustc must spell the lint the way rustc knows it — so this
/// fold has to be applied at *every* point where a name comes from a human:
/// `budget.toml` keys, `--allow`/`--warn`/`--deny`, and `--explain`.
///
/// It lives here rather than at any one call site because the bug it fixes (#487)
/// was exactly a split: `--explain Soroban_Storage_In_Loop` worked while the same
/// spelling in `budget.toml` was rejected as an unknown lint. One rule, one place.
///
/// Lower-casing is safe because no two lints differ only by case — `LINT_NAMES`
/// is built from lower-cased stems — so folding can never merge two distinct
/// lints. It *can* make one key appear twice, which the validators below reject.
pub fn normalize_lint_name(name: &str) -> String {
    name.to_lowercase()
}

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
    ///
    /// Keys are validated — and returned — in normalised form, so callers hand
    /// canonical lint names straight to rustc.
    ///
    /// Returns [`LinterError`] rather than a bare `String` so a caller can tell
    /// an unreadable file from a malformed one from an unknown lint (#485);
    /// `main()` treats all three as fatal, but they are distinct failures for
    /// anything that wants to report them differently.
    pub fn from_file_validated(path: &Path, known_lints: &[&str]) -> LinterResult<Self> {
        let content = fs::read_to_string(path).map_err(|source| LinterError::ConfigRead {
            path: path.to_path_buf(),
            source,
        })?;
        let mut config: BudgetConfig =
            toml::from_str(&content).map_err(|source| LinterError::ConfigParse {
                path: path.to_path_buf(),
                source,
            })?;

        let Some(lints) = config.lints.as_ref() else {
            return Ok(config);
        };

        let mut normalised: HashMap<String, String> = HashMap::with_capacity(lints.len());
        for (lint, level) in lints {
            let name = normalize_lint_name(lint);

            if !known_lints.contains(&name.as_str()) {
                return Err(LinterError::UnknownLintName {
                    name: lint.clone(),
                    path: path.to_path_buf(),
                    valid: known_lints.join(", "),
                });
            }
            if !matches!(level.as_str(), "allow" | "warn" | "deny") {
                return Err(LinterError::InvalidLintLevel {
                    level: level.clone(),
                    lint: lint.clone(),
                    path: path.to_path_buf(),
                });
            }
            // Two keys that differ only by case now name the same lint. If they
            // disagree on the level there is no correct choice between them, and
            // taking whichever came last would depend on hash-map order.
            if let Some(previous) = normalised.insert(name.clone(), level.clone())
                && previous != *level
            {
                return Err(LinterError::DuplicateLintName {
                    name,
                    path: path.to_path_buf(),
                });
            }
        }
        config.lints = Some(normalised);

        Ok(config)
    }

    /// Converts the lint severity settings into rustc-compatible flags for
    /// `DYLINT_RUSTFLAGS`.
    ///
    /// Each severity maps to a flag prefix:
    ///   "allow"  → `-A`     (allow the lint at module level)
    ///   "warn"   → `-W`     (upgrade to warning)
    ///   "deny"   → `-D`     (upgrade to error)
    // Superseded by `build_effective_lint_flags` in main.rs, which also folds in
    // the command-line overrides. Kept for its unit tests below.
    #[allow(dead_code)]
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
                _ => continue,
            };
            flags.push(format!("{} {}", prefix, lint_name));
        }
        flags
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
        assert!(config.to_lint_flags().is_empty());
    }

    #[test]
    fn from_file_validated_returns_error_for_missing_file() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("nonexistent.toml");
        let result = BudgetConfig::from_file_validated(&missing, KNOWN_LINTS);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read"));
    }

    #[test]
    fn read_and_parse_failures_are_distinct_variants() {
        // #485: callers must be able to tell "unreadable" from "malformed"
        // without matching on the message text.
        let dir = tempdir().unwrap();

        let missing = dir.path().join("nonexistent.toml");
        match BudgetConfig::from_file_validated(&missing, KNOWN_LINTS).unwrap_err() {
            LinterError::ConfigRead { path, .. } => assert_eq!(path, missing),
            other => panic!("expected ConfigRead, got {:?}", other),
        }

        let malformed = dir.path().join("budget.toml");
        write_file(&malformed, "this is not valid toml [[[");
        match BudgetConfig::from_file_validated(&malformed, KNOWN_LINTS).unwrap_err() {
            LinterError::ConfigParse { path, .. } => assert_eq!(path, malformed),
            other => panic!("expected ConfigParse, got {:?}", other),
        }
    }

    #[test]
    fn validation_failures_name_the_path_and_the_offending_value() {
        let dir = tempdir().unwrap();

        let unknown_lint = dir.path().join("budget.toml");
        write_file(&unknown_lint, "[lints]\nnot_a_real_lint = \"deny\"\n");
        match BudgetConfig::from_file_validated(&unknown_lint, KNOWN_LINTS).unwrap_err() {
            LinterError::UnknownLintName { name, path, valid } => {
                assert_eq!(name, "not_a_real_lint");
                assert_eq!(path, unknown_lint);
                assert!(valid.contains("soroban_storage_in_loop"));
            }
            other => panic!("expected UnknownLintName, got {:?}", other),
        }

        let bad_level = dir.path().join("budget.toml");
        write_file(&bad_level, "[lints]\nsoroban_storage_in_loop = \"oops\"\n");
        match BudgetConfig::from_file_validated(&bad_level, KNOWN_LINTS).unwrap_err() {
            LinterError::InvalidLintLevel { level, lint, path } => {
                assert_eq!(level, "oops");
                assert_eq!(lint, "soroban_storage_in_loop");
                assert_eq!(path, bad_level);
            }
            other => panic!("expected InvalidLintLevel, got {:?}", other),
        }
    }

    #[test]
    fn source_errors_are_reachable_for_callers_that_want_the_cause() {
        use std::error::Error;

        let dir = tempdir().unwrap();
        let missing = dir.path().join("nonexistent.toml");
        let err = BudgetConfig::from_file_validated(&missing, KNOWN_LINTS).unwrap_err();
        assert!(
            err.source().is_some(),
            "underlying io error should be a source"
        );
    }

    #[test]
    fn normalize_lint_name_folds_case_only() {
        assert_eq!(
            normalize_lint_name("Soroban_Storage_In_Loop"),
            "soroban_storage_in_loop"
        );
        assert_eq!(
            normalize_lint_name("soroban_storage_in_loop"),
            "soroban_storage_in_loop"
        );
        // Lint names are lower_snake_case, so nothing but case can be folded
        // away — a name that is not lower_snake_case must come back changed.
        assert_eq!(normalize_lint_name("not-a-lint"), "not-a-lint");
    }

    #[test]
    fn from_file_validated_accepts_any_casing_and_returns_canonical_keys() {
        // #487: the case folding `--explain` already did has to hold for
        // budget.toml too, and the caller has to receive the canonical
        // spelling so it never hands rustc a lint name rustc does not know.
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(
            &path,
            "[lints]\nSoroban_Storage_In_Loop = \"deny\"\nRedundant_Env_Clone = \"warn\"\n",
        );

        let config = BudgetConfig::from_file_validated(&path, KNOWN_LINTS).unwrap();
        let lints = config.lints.as_ref().expect("lints should be present");
        assert_eq!(
            lints.get("soroban_storage_in_loop").map(String::as_str),
            Some("deny")
        );
        assert_eq!(
            lints.get("redundant_env_clone").map(String::as_str),
            Some("warn")
        );
        for key in lints.keys() {
            assert_eq!(key, &normalize_lint_name(key), "keys must be canonical");
        }
        assert_eq!(
            config.to_lint_flags().len(),
            2,
            "flags must be built from the canonical names"
        );
        assert!(
            config
                .to_lint_flags()
                .contains(&"-D soroban_storage_in_loop".to_string())
        );
    }

    #[test]
    fn from_file_validated_rejects_case_variants_that_disagree_on_level() {
        // Folding case together two keys that set different levels: taking
        // either would depend on hash-map iteration order.
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(
            &path,
            "[lints]\nSoroban_Storage_In_Loop = \"deny\"\nsoroban_storage_in_loop = \"warn\"\n",
        );

        match BudgetConfig::from_file_validated(&path, KNOWN_LINTS).unwrap_err() {
            LinterError::DuplicateLintName { name, path: p } => {
                assert_eq!(name, "soroban_storage_in_loop");
                assert_eq!(p, path);
            }
            other => panic!("expected DuplicateLintName, got {:?}", other),
        }
    }

    #[test]
    fn from_file_validated_accepts_case_variants_that_agree_on_level() {
        // Same level, so there is nothing to choose between and no flag to
        // duplicate — the fold collapses them.
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(
            &path,
            "[lints]\nSoroban_Storage_In_Loop = \"deny\"\nsoroban_storage_in_loop = \"deny\"\n",
        );

        let config = BudgetConfig::from_file_validated(&path, KNOWN_LINTS).unwrap();
        let lints = config.lints.as_ref().expect("lints should be present");
        assert_eq!(lints.len(), 1);
        assert_eq!(
            lints.get("soroban_storage_in_loop").map(String::as_str),
            Some("deny")
        );
    }

    #[test]
    fn from_file_validated_returns_error_for_malformed_toml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(&path, "this is not valid toml [[[");
        let result = BudgetConfig::from_file_validated(&path, KNOWN_LINTS);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse"));
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
        let flags = config.to_lint_flags();
        assert_eq!(flags, vec!["-D soroban_storage_in_loop".to_string()]);
    }

    #[test]
    fn from_file_validated_handles_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(&path, "");
        let config = BudgetConfig::from_file_validated(&path, KNOWN_LINTS).unwrap();
        assert!(config.lints.is_none());
        assert!(config.to_lint_flags().is_empty());
    }

    #[test]
    fn from_file_validated_rejects_unknown_lint_name() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(&path, "[lints]\nnot_a_real_lint = \"deny\"\n");
        let result = BudgetConfig::from_file_validated(&path, KNOWN_LINTS);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown lint name")
        );
    }

    #[test]
    fn from_file_validated_rejects_unknown_level() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(&path, "[lints]\nsoroban_storage_in_loop = \"oops\"\n");
        let result = BudgetConfig::from_file_validated(&path, KNOWN_LINTS);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown lint level")
        );
    }

    #[test]
    fn to_lint_flags_maps_severity_to_rustc_flag() {
        let mut lints = HashMap::new();
        lints.insert("soroban_storage_in_loop".to_string(), "deny".to_string());
        lints.insert("redundant_env_clone".to_string(), "warn".to_string());
        let config = BudgetConfig { lints: Some(lints) };
        let flags = config.to_lint_flags();
        assert_eq!(flags.len(), 2);
        assert!(flags.contains(&"-D soroban_storage_in_loop".to_string()));
        assert!(flags.contains(&"-W redundant_env_clone".to_string()));
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
        assert!(
            config.to_lint_flags().is_empty(),
            "obsolete [budget] wrapper schema should produce no lint flags"
        );
    }
}
