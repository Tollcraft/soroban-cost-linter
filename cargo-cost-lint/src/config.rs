use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::error::{LinterError, LinterResult};

#[derive(Deserialize, Debug, Default, Clone)]
pub struct Config {
    lints: Option<HashMap<String, String>>,
}

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

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;

    use tempfile::tempdir;

    use super::*;

    fn write_file(path: &Path, contents: &str) {
        let mut f = File::create(path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
    }

    #[test]
    fn from_file_or_default_returns_default_for_missing_file() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("nonexistent.toml");
        let config = Config::from_file_or_default(&missing).unwrap();
        assert!(config.lints.is_none());
    }

    #[test]
    fn from_file_or_default_parses_valid_toml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(
            &path,
            r#"[lints]
soroban_storage_in_loop = "deny"
"#,
        );
        let config = Config::from_file_or_default(&path).unwrap();
        let lints = config.lints.expect("lints should be present");
        assert_eq!(
            lints.get("soroban_storage_in_loop").map(|s| s.as_str()),
            Some("deny")
        );
    }

    #[test]
    fn from_file_or_default_handles_malformed_toml_gracefully() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(&path, "this is not valid toml [[[");
        let result = Config::from_file_or_default(&path);
        assert!(result.is_err());
    }

    #[test]
    fn from_file_or_default_handles_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(&path, "");
        let config = Config::from_file_or_default(&path).unwrap();
        assert!(config.lints.is_none());
    }

    #[test]
    fn default_config_has_no_lints() {
        let config = Config::default();
        assert!(config.lints.is_none());
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
        lints.insert("unnecessary_host_function_call".to_string(), "allow".to_string());
        lints.insert("host_in_loop".to_string(), "forbid".to_string());
        let config = Config {
            lints: Some(lints),
        };
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
        let config = Config {
            lints: Some(lints),
        };
        let flags = config.to_lint_flags();
        assert_eq!(flags.len(), 1);
        assert!(flags.contains(&"-D valid_lint".to_string()));
    }
}
