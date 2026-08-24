use std::fmt;
use std::io;

/// Central error type for the `cargo-cost-lint` CLI tool.
///
/// All fallible operations in the CLI return `Result<T, LinterError>` so that
/// error handling is consistent and caller-friendly instead of mixing `unwrap`,
/// `expect`, `exit`, and ad‑hoc `eprintln!` calls.
#[derive(Debug)]
// `Subprocess` and `MissingPrerequisite` are part of the consolidated error
// surface (#299/#281) but nothing raises them yet.
// Kept: part of a consolidated error surface but currently unraised
#[allow(dead_code)]
pub enum LinterError {
    /// An I/O error — file read/write, pipe capture, etc.
    Io(io::Error),
    /// JSON (de)serialisation failed.
    Json(serde_json::Error),
    /// A child process (`cargo dylint`) exited with a non-zero status.
    Subprocess { code: Option<i32> },
    /// A required prerequisite is missing (e.g. `cargo-dylint` not installed).
    MissingPrerequisite(String),
    /// A generic, human-readable error message for unexpected situations.
    Other(String),
}

/// Convenience alias so every module can write `LinterResult<T>` instead of
/// `std::result::Result<T, LinterError>`.
pub type LinterResult<T> = std::result::Result<T, LinterError>;

impl fmt::Display for LinterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinterError::Io(e) => write!(f, "I/O error: {}", e),
            LinterError::Json(e) => write!(f, "JSON error: {}", e),
            LinterError::Subprocess { code } => write!(f, "subprocess exited with code {:?}", code),
            LinterError::MissingPrerequisite(msg) => write!(f, "{}", msg),
            LinterError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for LinterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LinterError::Io(e) => Some(e),
            LinterError::Json(e) => Some(e),
            _ => None,
        }
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
