use std::fmt;
use std::io;
use std::path::PathBuf;

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
    /// A `budget.toml` exists but could not be read.
    ///
    /// Split out from [`LinterError::Io`] so a caller can tell "the user's
    /// config is unreadable" from "some other file operation failed" without
    /// string matching — which is what `config.rs` used to force callers to do
    /// (`#485`).
    ConfigRead { path: PathBuf, source: io::Error },
    /// A `budget.toml` was read but is not valid TOML.
    ConfigParse {
        path: PathBuf,
        source: toml::de::Error,
    },
    /// A `budget.toml` names a lint that is not in the lint inventory.
    UnknownLintName {
        /// The name as the user wrote it, not the normalised form, so the
        /// message quotes back what they have to correct.
        name: String,
        path: PathBuf,
        /// The inventory, already joined: `Display` only has to print it.
        valid: String,
    },
    /// A `budget.toml` sets a level that is not `allow`, `warn` or `deny`.
    InvalidLintLevel {
        level: String,
        lint: String,
        path: PathBuf,
    },
    /// Two `budget.toml` keys name the same lint at different levels.
    ///
    /// Keys are matched case-insensitively, so this can only happen when two
    /// spellings of one lint disagree — picking either one silently would make
    /// the applied level depend on hash-map iteration order.
    DuplicateLintName { name: String, path: PathBuf },
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
            LinterError::ConfigRead { path, source } => {
                write!(f, "Error: Failed to read {}: {}", path.display(), source)
            }
            LinterError::ConfigParse { path, source } => {
                write!(f, "Error: Failed to parse {}: {}", path.display(), source)
            }
            LinterError::UnknownLintName { name, path, valid } => write!(
                f,
                "Error: Unknown lint name '{}' in {}. Valid lints: {}",
                name,
                path.display(),
                valid
            ),
            LinterError::InvalidLintLevel { level, lint, path } => write!(
                f,
                "Error: Unknown lint level '{}' for '{}' in {}. Valid levels are allow, warn, and deny.",
                level,
                lint,
                path.display()
            ),
            LinterError::DuplicateLintName { name, path } => write!(
                f,
                "Error: Conflicting levels for '{}' in {}: the same lint is spelled two ways \
                 with different levels; keep one spelling",
                name,
                path.display()
            ),
            LinterError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for LinterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LinterError::Io(e) => Some(e),
            LinterError::Json(e) => Some(e),
            LinterError::ConfigRead { source, .. } => Some(source),
            LinterError::ConfigParse { source, .. } => Some(source),
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
