use std::env;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug)]
enum Error {
    Io(std::io::Error),
    MissingEnv,
    Parse(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::MissingEnv => write!(f, "OUT_DIR environment variable not set"),
            Error::Parse(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

type Result<T> = std::result::Result<T, Error>;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    println!("cargo:rerun-if-changed=../soroban_cost_lints/src/lib.rs");

    let content = fs::read_to_string("../soroban_cost_lints/src/lib.rs")?;

    let start_marker = "lint_store.register_lints(&[";
    let start = content
        .find(start_marker)
        .ok_or_else(|| Error::Parse("Could not find register_lints in lib.rs".into()))?;
    let content_after = &content[start..];
    let end = content_after
        .find("]);")
        .ok_or_else(|| Error::Parse("Could not find end of register_lints".into()))?;

    let list_str = &content_after[start_marker.len()..end];

    let mut names = Vec::new();
    for line in list_str.lines() {
        let trimmed = line.trim().trim_end_matches(',');
        if !trimmed.is_empty() && !trimmed.starts_with("//") {
            names.push(trimmed.to_lowercase());
        }
    }

    let out_dir = env::var_os("OUT_DIR").ok_or(Error::MissingEnv)?;
    let dest_path = Path::new(&out_dir).join("lint_names.rs");

    let mut out = String::new();
    out.push_str("pub const LINT_NAMES: &[&str] = &[\n");
    for name in names {
        out.push_str(&format!("    \"{}\",\n", name));
    }
    out.push_str("];\n");

    fs::write(&dest_path, out)?;

    Ok(())
}
