use std::path::Path;
use std::process::Command;

use crate::error::{LinterError, LinterResult};

/// Get the list of files changed relative to the merge base with the default
/// branch. Returns paths relative to the repository root.
///
/// # Errors
///
/// Returns an error if:
/// - The current directory is not inside a git repository
/// - The default branch cannot be determined
/// - There is no common ancestor (merge base) with the default branch
pub fn get_changed_files() -> LinterResult<Vec<String>> {
    let default_branch = detect_default_branch()?;
    let merge_base = find_merge_base(&default_branch)?;
    get_diff_files(&merge_base)
}

/// Detect the repository's default branch by checking:
/// 1. The `origin/HEAD` reference
/// 2. Falls back to `main`, then `master`
fn detect_default_branch() -> LinterResult<String> {
    // Try origin/HEAD first
    let output = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .output()
        .map_err(|e| LinterError::Other(format!("Failed to run git: {e}")))?;

    if output.status.success() {
        let ref_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // refs/remotes/origin/main -> main
        if let Some(branch) = ref_name.strip_prefix("refs/remotes/origin/") {
            return Ok(branch.to_string());
        }
    }

    // Try common defaults
    for candidate in &["main", "master"] {
        let check = Command::new("git")
            .args(["rev-parse", "--verify", &format!("origin/{candidate}")])
            .output();
        if let Ok(o) = check
            && o.status.success()
        {
            return Ok(candidate.to_string());
        }
    }

    Err(LinterError::Other(
        "Could not determine the default branch. \
         Ensure 'origin/HEAD', 'origin/main', or 'origin/master' exists."
            .to_string(),
    ))
}

/// Find the merge base between the current HEAD and the given branch.
fn find_merge_base(default_branch: &str) -> LinterResult<String> {
    let output = Command::new("git")
        .args(["merge-base", "HEAD", &format!("origin/{default_branch}")])
        .output()
        .map_err(|e| LinterError::Other(format!("Failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LinterError::Other(format!(
            "No common ancestor found between HEAD and origin/{default_branch}.\n\
             {stderr}"
        )));
    }

    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(sha)
}

/// Get the list of files changed between the given base SHA and HEAD.
fn get_diff_files(base: &str) -> LinterResult<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--name-only", base, "HEAD"])
        .output()
        .map_err(|e| LinterError::Other(format!("Failed to run git: {e}")))?;

    if !output.status.success() {
        return Err(LinterError::Other(
            "Failed to get diff files from git".to_string(),
        ));
    }

    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.trim().to_string())
        .collect();

    Ok(files)
}

/// Whether `file` is one of the `changed` paths, comparing relative to `root`.
///
/// Shared by [`filter_files_by_changed`] and the per-finding `--diff-only`
/// filter in `main.rs`, so a finding and a file are judged by the same rule.
pub fn is_file_changed(file: &str, changed: &[String], root: &Path) -> bool {
    let path = Path::new(file);
    let relative = path.strip_prefix(root).unwrap_or(path);
    let relative_str = relative.to_string_lossy();

    changed.iter().any(|c| {
        relative_str == c.as_str()
            || Path::new(c)
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| relative_str.ends_with(n))
    })
}

/// Filter a list of file paths to only those in the given `changed` set.
///
/// The `--diff-only` path filters findings one at a time through
/// [`is_file_changed`]; this list-level form is the module's public API and is
/// covered by the tests below, so it is kept rather than deleted.
/// Paths are compared relative to the given root directory.
#[allow(dead_code)]
pub fn filter_files_by_changed(files: &[String], changed: &[String], root: &Path) -> Vec<String> {
    files
        .iter()
        .filter(|file| is_file_changed(file, changed, root))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_filter_files_by_changed() {
        let root = PathBuf::from("/workspace");
        let files = vec![
            "src/lib.rs".to_string(),
            "src/other.rs".to_string(),
            "tests/test.rs".to_string(),
        ];
        let changed = vec!["src/lib.rs".to_string()];

        let filtered = filter_files_by_changed(&files, &changed, &root);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0], "src/lib.rs");
    }

    #[test]
    fn test_filter_empty_changed() {
        let root = PathBuf::from("/workspace");
        let files = vec!["src/lib.rs".to_string()];
        let changed: Vec<String> = vec![];

        let filtered = filter_files_by_changed(&files, &changed, &root);
        assert!(filtered.is_empty());
    }
}
