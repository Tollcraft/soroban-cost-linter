use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::error::{LinterError, LinterResult};

#[derive(Deserialize, Debug, Default, Clone)]
pub struct Config {
    #[allow(dead_code)]
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
}
