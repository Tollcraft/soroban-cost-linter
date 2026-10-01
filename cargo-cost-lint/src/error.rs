use std::fmt;
use std::io;

/// Central error type for the `cargo-cost-lint` CLI tool.
///
/// All fallible operations in the CLI return `Result<T, LinterError>` so that
/// error handling is consistent and caller-friendly instead of mixing `unwrap`,
/// `expect`, `exit`, and ad‑hoc `eprintln!` calls.
#[derive(Debug)]
pub enum LinterError {
    /// An I/O error — file read/write, pipe capture, etc.
    Io(io::Error),
    /// JSON (de)serialisation failed.
    Json(serde_json::Error),
    /// A child process (`cargo dylint`) exited with a non-zero status.
    // main() reports subprocess exits by calling `exit` with the child's code
    // directly, so nothing constructs this today; kept as part of the public
    // error taxonomy.
    #[allow(dead_code)]
    Subprocess { code: Option<i32> },
    /// A required prerequisite is missing (e.g. `cargo-dylint` not installed).
    MissingPrerequisite(String),
    /// A generic, human-readable error message for unexpected situations.
    Other(String),
}

/// Convenience alias so every module can write `LinterResult<T>` instead of
/// `std::result::Result<T, LinterError>`.
pub type LinterResult<T> = std::result::Result<T, LinterError>;

impl LinterError {
    fn write_display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {}", error),
            Self::Json(error) => write!(f, "JSON error: {}", error),
            Self::Subprocess { code } => write!(f, "subprocess exited with code {:?}", code),
            Self::MissingPrerequisite(message) | Self::Other(message) => f.write_str(message),
        }
    }

    fn source_error(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Subprocess { .. } | Self::MissingPrerequisite(_) | Self::Other(_) => None,
        }
    }
}

impl fmt::Display for LinterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_display(f)
    }
}

impl std::error::Error for LinterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source_error()
    }
}

// ---------------------------------------------------------------------------
// From conversions — allows `?` to work ergonomically with common types
// ---------------------------------------------------------------------------

impl From<io::Error> for LinterError {
    fn from(e: io::Error) -> Self {
        LinterError::Io(e)
    }
}

impl From<serde_json::Error> for LinterError {
    fn from(e: serde_json::Error) -> Self {
        LinterError::Json(e)
    }
}

impl From<String> for LinterError {
    fn from(s: String) -> Self {
        LinterError::Other(s)
    }
}

impl From<&str> for LinterError {
    fn from(s: &str) -> Self {
        LinterError::Other(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn display_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = LinterError::Io(io_err);
        let msg = format!("{}", err);
        assert!(msg.contains("I/O error"));
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn display_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err = LinterError::Json(json_err);
        let msg = format!("{}", err);
        assert!(msg.contains("JSON error"));
    }

    #[test]
    fn display_subprocess_error_with_code() {
        let err = LinterError::Subprocess { code: Some(42) };
        let msg = format!("{}", err);
        assert!(msg.contains("subprocess exited with code"));
        assert!(msg.contains("42"));
    }

    #[test]
    fn display_subprocess_error_without_code() {
        let err = LinterError::Subprocess { code: None };
        let msg = format!("{}", err);
        assert!(msg.contains("subprocess exited with code None"));
    }

    #[test]
    fn display_missing_prerequisite() {
        let err = LinterError::MissingPrerequisite("cargo-dylint not installed".to_string());
        let msg = format!("{}", err);
        assert_eq!(msg, "cargo-dylint not installed");
    }

    #[test]
    fn display_other_error() {
        let err = LinterError::Other("something went wrong".to_string());
        let msg = format!("{}", err);
        assert_eq!(msg, "something went wrong");
    }

    #[test]
    fn source_returns_inner_for_io() {
        let io_err = io::Error::other("test");
        let err = LinterError::Io(io_err);
        assert!(err.source().is_some());
    }

    #[test]
    fn source_returns_inner_for_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err = LinterError::Json(json_err);
        assert!(err.source().is_some());
    }

    #[test]
    fn source_returns_none_for_other() {
        let err = LinterError::Other("test".to_string());
        assert!(err.source().is_none());
    }

    #[test]
    fn source_returns_none_for_missing_prerequisite() {
        let err = LinterError::MissingPrerequisite("test".to_string());
        assert!(err.source().is_none());
    }

    #[test]
    fn source_returns_none_for_subprocess() {
        let err = LinterError::Subprocess { code: Some(1) };
        assert!(err.source().is_none());
    }

    #[test]
    fn from_io_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        let err: LinterError = io_err.into();
        match err {
            LinterError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::PermissionDenied),
            _ => panic!("expected Io variant"),
        }
    }

    #[test]
    fn from_json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("bad").unwrap_err();
        let err: LinterError = json_err.into();
        match err {
            LinterError::Json(_) => {}
            _ => panic!("expected Json variant"),
        }
    }

    #[test]
    fn from_string_conversion() {
        let err: LinterError = "error message".to_string().into();
        match err {
            LinterError::Other(msg) => assert_eq!(msg, "error message"),
            _ => panic!("expected Other variant"),
        }
    }

    #[test]
    fn from_str_conversion() {
        let err: LinterError = "error message".into();
        match err {
            LinterError::Other(msg) => assert_eq!(msg, "error message"),
            _ => panic!("expected Other variant"),
        }
    }

    #[test]
    fn error_trait_impl() {
        let err = LinterError::Other("test".to_string());
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn debug_formatting() {
        let err = LinterError::Other("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Other"));
    }

    #[test]
    fn linter_result_ok() {
        let result: LinterResult<i32> = Ok(42);
        assert!(result.is_ok());
    }

    #[test]
    fn linter_result_err() {
        let result: LinterResult<i32> = Err(LinterError::Other("fail".to_string()));
        assert!(result.is_err());
    }
}
