use std::path::{Path, PathBuf};
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

/// Whether `file` is one of the `changed` paths, comparing repository-relative
/// paths after normalising separators and resolving `..` segments.
///
/// Shared by [`filter_files_by_changed`] and the per-finding `--diff-only`
/// filter in `main.rs`, so a finding and a file are judged by the same rule.
/// A basename match would make editing package A's `src/lib.rs` un-filter
/// package B's findings in every other `src/lib.rs` of the workspace — the
/// exact opposite of what `--diff-only` exists for (#794) — so only the full
/// path is compared.
pub fn is_file_changed(file: &str, changed: &[String], root: &Path) -> bool {
    let normalised = normalise_relative_path(file, root);
    let changed_paths: Vec<String> = changed
        .iter()
        .map(|c| normalise_relative_path(c, root))
        .collect();
    changed_paths.contains(&normalised)
}

/// Reduces a path to a repository-relative, forward-slash-separated form with
/// `.` and `..` segments resolved against `root`.
fn normalise_relative_path(path: &str, root: &Path) -> String {
    // Preserve a leading `/` so an absolute path can still be stripped of the
    // root prefix below.
    let absolute = path.starts_with('/');
    let cleaned: PathBuf = path
        .replace('\\', "/")
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != ".")
        .collect();
    // Restore the leading slash for absolute paths, strip the `root` prefix
    // when the path is absolute or rooted at `root`, then resolve `..`
    // segments within the remainder.
    let cleaned = cleaned
        .to_str()
        .map(|c| {
            if absolute {
                format!("/{c}")
            } else {
                c.to_string()
            }
        })
        .unwrap_or_default();
    let cleaned = PathBuf::from(&cleaned);
    let stripped = cleaned.strip_prefix(root).unwrap_or(&cleaned);
    let mut segments: Vec<String> = Vec::new();
    for seg in stripped.iter() {
        match seg.to_str() {
            Some("..") => {
                segments.pop();
            }
            // A leading `..` on the remainder is dropped instead of turning
            // into a traversal that would never match a changed file.
            _ => segments.push(seg.to_string_lossy().into_owned()),
        }
    }
    segments.join("/")
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

    #[test]
    fn workspace_same_basename_does_not_cross_match() {
        // Two packages both own src/lib.rs: editing A's lib.rs must never
        // un-filter B's findings (#794).
        let root = PathBuf::from("/workspace");
        let files = vec![
            "package_a/src/lib.rs".to_string(),
            "package_b/src/lib.rs".to_string(),
        ];
        let changed = vec!["package_a/src/lib.rs".to_string()];

        let filtered = filter_files_by_changed(&files, &changed, &root);
        assert_eq!(filtered, vec!["package_a/src/lib.rs".to_string()]);

        // A change to the owning file does include the finding, with the
        // absolute form normalised to the repository-relative one.
        assert!(is_file_changed(
            "/workspace/package_b/src/lib.rs",
            &changed_path_b(),
            &PathBuf::from("/workspace")
        ));
    }

    fn changed_path_b() -> Vec<String> {
        vec!["package_b/src/lib.rs".to_string()]
    }

    #[test]
    fn dotdot_and_backslash_segments_normalise() {
        let root = PathBuf::from("/workspace");
        let changed = vec!["package_b/src/lib.rs".to_string()];

        // Intra-path `..` is resolved against the remainder.
        assert!(is_file_changed(
            "package_b/../package_b/src/lib.rs",
            &changed,
            &root
        ));
        // Windows separators.
        assert!(is_file_changed(r"package_b\src\lib.rs", &changed, &root));
        // A leading `..` is dropped (see the helper's comment), so it cannot
        // accidentally cross-match another package's finding.
        assert!(!is_file_changed(r"package_c/src/lib.rs", &changed, &root));
    }
}
