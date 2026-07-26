use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Deserialize, Debug, Default, Clone)]
pub struct Config {
    #[allow(dead_code)]
    lints: Option<HashMap<String, String>>,
}

impl Config {
    pub fn from_file_or_default(path: &Path) -> Self {
        if !path.exists() {
            return Config::default();
        }
        fs::read_to_string(path)
            .ok()
            .and_then(|content| toml::from_str::<Config>(&content).ok())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    fn write_file(path: &Path, contents: &str) {
        let mut f = File::create(path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
    }

    #[test]
    fn from_file_or_default_returns_default_for_missing_file() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("nonexistent.toml");
        let config = Config::from_file_or_default(&missing);
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
        let config = Config::from_file_or_default(&path);
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
        let config = Config::from_file_or_default(&path);
        assert!(config.lints.is_none());
    }

    #[test]
    fn from_file_or_default_handles_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("budget.toml");
        write_file(&path, "");
        let config = Config::from_file_or_default(&path);
        assert!(config.lints.is_none());
    }

    #[test]
    fn default_config_has_no_lints() {
        let config = Config::default();
        assert!(config.lints.is_none());
    }
}
