//! Registry self-consistency invariants for `soroban_cost_lints/src/lib.rs`
//! (issue #879).
//!
//! Seventeen concurrent lint PRs each edit the same insertion points in
//! `lib.rs`, and a lost or mis-ordered entry lands silently in a rebase.
//! These tests parse `lib.rs` as text — the way `meta_test.rs` already does —
//! and assert that the registry agrees with itself:
//!
//! * the `rustc_session::declare_lint!` blocks, the
//!   `lint_store.register_lints(&[...])` array, the `dylint_lint_impl!`
//!   aggregate list and the `LintMeta` table must all hold the same set of
//!   lint names;
//! * every `.rs` file under `soroban_cost_lints/src/` other than `lib.rs`
//!   must have a `mod` declaration;
//! * the `mod` list must stay in sorted order.
//!
//! `register_late_pass` is deliberately *not* compared: not every lint has a
//! late-pass call, and that gap belongs to #785. Nothing here writes a
//! registry line.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path
}

fn lib_rs() -> String {
    let path = workspace_dir().join("soroban_cost_lints/src/lib.rs");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()))
}

/// One of the four registry insertion points, named so failures tell a
/// contributor exactly which site to fix.
struct Site {
    name: &'static str,
    lints: Vec<String>,
}

impl Site {
    fn lowercased(&self) -> BTreeSet<String> {
        self.lints.iter().cloned().collect()
    }

    fn duplicates(&self) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut dupes = BTreeSet::new();
        for lint in &self.lints {
            if !seen.insert(lint.clone()) {
                dupes.insert(lint.clone());
            }
        }
        dupes
    }
}

fn clean_line(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches(',').trim();
    if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
        None
    } else {
        Some(trimmed.to_lowercase())
    }
}

/// Names from the `lint_store.register_lints(&[...])` array. The first
/// occurrence of the marker is the real array; the later one is a string
/// literal inside a `#[test]` fn further down `lib.rs`.
fn parse_register_lints_array(content: &str) -> Vec<String> {
    const MARKER: &str = "lint_store.register_lints(&[";
    let start = content
        .find(MARKER)
        .expect("lib.rs must contain `lint_store.register_lints(&[`");
    let after = &content[start + MARKER.len()..];
    let end = after
        .find("]);")
        .expect("unterminated `lint_store.register_lints(&[` array in lib.rs");
    after[..end].lines().filter_map(clean_line).collect()
}

/// Names from the `dylint_lint_impl! { Pass, [ ... ] }` aggregate list.
fn parse_dylint_lint_impl_list(content: &str) -> Vec<String> {
    const MARKER: &str = "dylint_lint_impl! {";
    let start = content
        .find(MARKER)
        .expect("lib.rs must contain a `dylint_lint_impl! {` invocation");
    let after = &content[start + MARKER.len()..];
    let open = after
        .find('[')
        .expect("`dylint_lint_impl!` call has no `[` lint list");
    let close = after[open + 1..]
        .find(']')
        .expect("unterminated `dylint_lint_impl!` lint list");
    after[open + 1..open + 1 + close]
        .lines()
        .filter_map(clean_line)
        .collect()
}

/// Names from the `rustc_session::declare_lint! { pub NAME, ... }` blocks.
fn parse_declare_lint_blocks(content: &str) -> Vec<String> {
    const MARKER: &str = "rustc_session::declare_lint! {";
    let mut names = Vec::new();
    let mut offset = 0;
    while let Some(rel) = content[offset..].find(MARKER) {
        let after = &content[offset + rel + MARKER.len()..];
        let name = after
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with("//"))
            .unwrap_or_else(|| {
                panic!(
                    "empty `declare_lint!` block for entry #{names_len}",
                    names_len = names.len() + 1
                )
            });
        let Some(rest) = name.strip_prefix("pub ") else {
            panic!("`declare_lint!` block must open with `pub NAME,`, found: {name}");
        };
        let lint = rest.trim().trim_end_matches(',').to_lowercase();
        assert!(
            !lint.is_empty()
                && lint
                    .replace('_', "")
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            "`declare_lint!` block has a non-identifier name: {name}"
        );
        names.push(lint);
        offset += rel + MARKER.len();
    }
    names
}

/// `name: "..."` strings from the `pub const LINT_METADATA: &[LintMeta]`
/// table.
fn parse_lint_meta_table(content: &str) -> Vec<String> {
    const MARKER: &str = "pub const LINT_METADATA";
    let start = content
        .find(MARKER)
        .expect("lib.rs must contain `pub const LINT_METADATA`");
    let after = &content[start..];
    let end = after
        .find("];")
        .expect("unterminated `LINT_METADATA` table in lib.rs");
    let table = &after[..end];
    const FIELD: &str = "name: \"";
    let mut names = Vec::new();
    let mut offset = 0;
    while let Some(rel) = table[offset..].find(FIELD) {
        let rest = &table[offset + rel + FIELD.len()..];
        let close = rest
            .find('"')
            .expect("unterminated `name: \"...\"` string in a LintMeta row");
        names.push(rest[..close].to_lowercase());
        offset += rel + FIELD.len() + close;
    }
    names
}

/// `mod NAME;` declarations in `lib.rs`, in source order.
fn parse_mod_declarations(content: &str) -> Vec<String> {
    let mut mods = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("mod ")
            && let Some(name) = rest.strip_suffix(';')
            && !name.contains(' ')
        {
            mods.push(name.to_lowercase());
        }
    }
    mods
}

/// Every `.rs` file under `soroban_cost_lints/src/` except `lib.rs`,
/// recursively, as `(stem, path)` pairs.
fn collect_source_files(dir: &Path, files: &mut Vec<(String, PathBuf)>) {
    for entry in
        fs::read_dir(dir).unwrap_or_else(|e| panic!("Failed to read {}: {e}", dir.display()))
    {
        let entry = entry.expect("readable directory entry");
        let path = entry.path();
        if path.is_dir() {
            collect_source_files(&path, files);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs")
            && path.file_name().and_then(|e| e.to_str()) != Some("lib.rs")
        {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .expect("utf-8 file stem")
                .to_lowercase();
            files.push((stem, path));
        }
    }
}

fn registry_sites(content: &str) -> Vec<Site> {
    let sites = vec![
        Site {
            name: "`rustc_session::declare_lint!` blocks",
            lints: parse_declare_lint_blocks(content),
        },
        Site {
            name: "`lint_store.register_lints(&[...])` array",
            lints: parse_register_lints_array(content),
        },
        Site {
            name: "`dylint_lint_impl!` aggregate list",
            lints: parse_dylint_lint_impl_list(content),
        },
        Site {
            name: "`LintMeta` table (`pub const LINT_METADATA`)",
            lints: parse_lint_meta_table(content),
        },
    ];
    for site in &sites {
        assert!(
            !site.lints.is_empty(),
            "registry site {name} parsed empty — the parser or the marker moved; \
             a passing test on four empty sets would be vacuous",
            name = site.name
        );
    }
    sites
}

/// The four registry sites must name exactly the same set of lints, and no
/// site may list a lint twice. A lost or mis-ordered entry from a concurrent
/// rebase of `lib.rs` fails here with the lint name and the offending site.
#[test]
fn registry_sites_agree() {
    let content = lib_rs();
    let sites = registry_sites(&content);

    let mut duplicates = Vec::new();
    for site in &sites {
        for lint in site.duplicates() {
            duplicates.push(format!(
                "'{lint}' listed more than once in {name}",
                name = site.name
            ));
        }
    }
    assert!(
        duplicates.is_empty(),
        "the lint registry disagrees with itself:\n  {}",
        duplicates.join("\n  ")
    );

    // For each lint name, which sites do NOT mention it.
    let mut missing: HashMap<String, Vec<&'static str>> = HashMap::new();
    for site in &sites {
        let names = site.lowercased();
        let all: BTreeSet<String> = sites
            .iter()
            .flat_map(|s| s.lowercased())
            .chain(names.iter().cloned())
            .collect();
        for lint in all {
            if !names.contains(&lint) {
                missing.entry(lint).or_default().push(site.name);
            }
        }
    }
    assert!(
        missing.is_empty(),
        "the lint registry disagrees with itself — each line names a lint and the \
         site(s) it is missing from:\n  {}",
        BTreeSet::from_iter(
            missing.iter().map(|(lint, site_names)| format!(
                "'{lint}' missing from: {}",
                site_names.join(", ")
            ))
        )
        .into_iter()
        .collect::<Vec<_>>()
        .join("\n  ")
    );
}

/// Every `.rs` file under `soroban_cost_lints/src/` other than `lib.rs` must
/// have a `mod` declaration in `lib.rs` (and vice versa, so a `mod` line
/// without a file fails too). A `dir/mod.rs` file is satisfied by a
/// `mod dir;` declaration.
#[test]
fn every_lint_source_file_has_a_mod_declaration() {
    let ws = workspace_dir();
    let src_dir = ws.join("soroban_cost_lints/src");
    let mods: BTreeSet<String> = BTreeSet::from_iter(parse_mod_declarations(&lib_rs()));

    let mut files = Vec::new();
    collect_source_files(&src_dir, &mut files);
    assert!(
        !files.is_empty(),
        "found no .rs files under {} — the walker or the layout moved",
        src_dir.display()
    );

    let mut orphans = Vec::new();
    for (stem, path) in &files {
        // A `dir/mod.rs` file is the module directory form and is declared by
        // `mod dir;`; every other file is declared by `mod <stem>;`.
        let declared_by = if stem == "mod" {
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(|n| n.to_lowercase())
                .unwrap_or_default()
        } else {
            stem.clone()
        };
        if !mods.contains(&declared_by) {
            orphans.push(format!(
                "{} is not declared: add `mod {declared_by};` to soroban_cost_lints/src/lib.rs",
                path.strip_prefix(&ws).unwrap_or(path.as_path()).display()
            ));
        }
    }
    assert!(
        orphans.is_empty(),
        "orphan lint source files found (issue #879 asserts every .rs file under \
         soroban_cost_lints/src/ is a declared module):\n  {}",
        orphans.join("\n  ")
    );

    // Conversely: each `mod x;` needs a top-level `x.rs`, unless it is a
    // directory module `x/mod.rs`.
    let stems: BTreeSet<String> = files.iter().map(|(stem, _)| stem.clone()).collect();
    let dir_modules: BTreeSet<String> = files
        .iter()
        .filter(|(stem, _)| stem == "mod")
        .filter_map(|(_, p)| {
            p.parent()
                .and_then(|d| d.file_name())
                .and_then(|n| n.to_str())
                .map(|n| n.to_lowercase())
        })
        .collect();
    let unbacked: Vec<&String> = mods
        .iter()
        .filter(|m| !stems.contains(*m) && !dir_modules.contains(*m))
        .collect();
    assert!(
        unbacked.is_empty(),
        "`mod` declarations in lib.rs with no backing source file: {unbacked:?}"
    );
}

/// The `mod` list in `lib.rs` must stay in sorted order, so seventeen
/// concurrent lint PRs get deterministic conflicts instead of silent
/// reordering.
#[test]
fn mod_list_is_sorted() {
    let mods = parse_mod_declarations(&lib_rs());
    assert!(
        !mods.is_empty(),
        "parsed zero `mod` declarations — the parser or the marker moved"
    );
    let unsorted: Vec<String> = mods
        .windows(2)
        .filter(|w| w[0] > w[1])
        .map(|w| format!("`mod {0};` appears before `mod {1};`", w[0], w[1]))
        .collect();
    assert!(
        unsorted.is_empty(),
        "the `mod` list in soroban_cost_lints/src/lib.rs is not sorted:\n  {}",
        unsorted.join("\n  ")
    );
}
