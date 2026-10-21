//! The registration points a lint has to touch, pinned to the code.
//!
//! `DEVELOPING_LINTS.md` tells a contributor that a lint fires because of the
//! `lint_store.register_late_pass(...)` call inside `register_lints` — not
//! because of the `dylint_lint_impl!` list, which only implements
//! `rustc_session::impl_lint_pass!` for a marker struct the lint store never
//! sees.
//!
//! That distinction is invisible to `cargo-cost-lint/build.rs`, which
//! cross-checks the `dylint_lint_impl!` list, the `register_lints(&[..])` list
//! and `LINT_METADATA` against each other but cannot see which passes the
//! compiler actually runs. A lint that is declared, listed in every registry
//! and documented, yet never wired to a registered pass, is therefore a lint
//! that silently emits nothing.
//!
//! These tests close that gap: every registered lint must be driven by a pass,
//! and every pass that drives a lint must be registered with the lint store.

use std::collections::{BTreeMap, BTreeSet};
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

fn developing_lints_md() -> String {
    let path = workspace_dir().join("DEVELOPING_LINTS.md");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()))
}

/// Every `.rs` file under `soroban_cost_lints/src/`, recursively.
fn collect_source_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(dir).unwrap_or_else(|e| panic!("Failed to read {}: {e}", dir.display()))
    {
        let entry = entry.expect("readable directory entry");
        let path = entry.path();
        if path.is_dir() {
            collect_source_files(&path, files);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

/// True for a plain Rust identifier, used to skip the macro *definitions*
/// (`impl_lint_pass!($pass => [$($lint),*])`) that carry `$`-placeholders
/// rather than real names.
fn is_identifier(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !name.starts_with(|c: char| c.is_ascii_digit())
}

/// Lint names from the `lint_store.register_lints(&[...])` array in `lib.rs`.
fn parse_registered_lints(content: &str) -> BTreeSet<String> {
    const MARKER: &str = "lint_store.register_lints(&[";
    let start = content
        .find(MARKER)
        .expect("lib.rs must contain `lint_store.register_lints(&[`");
    let after = &content[start + MARKER.len()..];
    let end = after
        .find("]);")
        .expect("unterminated `lint_store.register_lints(&[` array in lib.rs");
    after[..end]
        .lines()
        .map(|line| line.trim().trim_end_matches(',').trim().to_lowercase())
        .filter(|name| !name.is_empty() && !name.starts_with("//"))
        .collect()
}

/// Pass struct names handed to the lint store by a `register_late_pass` call.
///
/// The call may spell the struct inline (`Box::new(Pass)`), qualified
/// (`Box::new(module::Pass)`) or constructed (`Box::new(Pass::new(..))`), so
/// only the last path segment is kept.
fn parse_registered_pass_structs(content: &str) -> BTreeSet<String> {
    const MARKER: &str = "register_late_pass";
    let mut names = BTreeSet::new();
    let mut offset = 0;
    while let Some(rel) = content[offset..].find(MARKER) {
        let start = offset + rel;
        let after = &content[start + MARKER.len()..];
        // The call body ends at the next registration call, so a `Box::new`
        // belonging to an earlier statement is never picked up.
        let end = after
            .find("register_late_pass")
            .or_else(|| after.find("register_early_pass"))
            .unwrap_or(after.len());
        let call = &after[..end];
        if let Some(pos) = call.find("Box::new(") {
            let tail = &call[pos + "Box::new(".len()..];
            let path: String = tail
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == ':')
                .collect();
            // The path may end in a constructor call (`Pass::new(..)`,
            // `Pass::default()`), so the type is the last segment that starts
            // with an upper-case letter.
            let struct_name = path
                .split("::")
                .filter(|segment| segment.chars().next().is_some_and(|c| c.is_uppercase()))
                .last();
            if let Some(name) = struct_name
                && is_identifier(name)
            {
                names.insert(name.to_string());
            }
        }
        offset = start + MARKER.len();
    }
    names
}

/// Pass struct name -> the lints it drives, from every
/// `declare_lint_pass!(Pass => [LINT, ..])` and
/// `impl_lint_pass!(Pass => [LINT, ..])` invocation under
/// `soroban_cost_lints/src/`.
fn parse_pass_declarations() -> BTreeMap<String, BTreeSet<String>> {
    let src_dir = workspace_dir().join("soroban_cost_lints/src");
    let mut files = Vec::new();
    collect_source_files(&src_dir, &mut files);
    assert!(
        !files.is_empty(),
        "found no .rs files under {} — the walker or the layout moved",
        src_dir.display()
    );

    let mut passes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for path in files {
        let content = fs::read_to_string(&path).expect("readable Rust source");
        for marker in ["declare_lint_pass!(", "impl_lint_pass!("] {
            let mut offset = 0;
            while let Some(rel) = content[offset..].find(marker) {
                let start = offset + rel + marker.len();
                let after = &content[start..];
                let Some(arrow) = after.find("=>") else {
                    break;
                };
                let name = after[..arrow].trim().to_string();
                let rest = &after[arrow + 2..];
                let Some(open) = rest.find('[') else {
                    break;
                };
                let Some(close) = rest[open..].find(']') else {
                    break;
                };
                if is_identifier(&name) {
                    let lints: BTreeSet<String> = rest[open + 1..open + close]
                        .split(',')
                        .map(str::trim)
                        // Lint statics are `SCREAMING_CASE`; anything else in
                        // the list is not a lint name.
                        .filter(|lint| {
                            is_identifier(lint) && lint.chars().all(|c| !c.is_lowercase())
                        })
                        .map(|lint| lint.to_lowercase())
                        .collect();
                    passes.entry(name).or_default().extend(lints);
                }
                offset = start;
            }
        }
    }
    passes
}

/// Every registered lint must be driven by a pass.
///
/// A lint that no pass drives is known to `rustc` and listed by the CLI, but
/// no diagnostic is ever emitted — the failure mode `DEVELOPING_LINTS.md`
/// warns about.
#[test]
fn every_registered_lint_is_driven_by_a_pass() {
    let registered = parse_registered_lints(&lib_rs());
    assert!(
        !registered.is_empty(),
        "parsed zero registered lints — the parser or the marker moved"
    );

    let passes = parse_pass_declarations();
    let driven: BTreeSet<String> = passes.values().flatten().cloned().collect();

    let undriven: Vec<&String> = registered
        .iter()
        .filter(|lint| !driven.contains(*lint))
        .collect();
    assert!(
        undriven.is_empty(),
        "registered lint(s) with no pass: {:?}. A lint is driven by a \
         `declare_lint_pass!`/`impl_lint_pass!` declaration naming it; without one \
         nothing ever reports it.",
        undriven
    );
}

/// Every pass that drives a lint must be handed to the lint store.
///
/// This is the step `cargo-cost-lint/build.rs` cannot check, and the one the
/// guide calls out as the difference between "the build tells you" and
/// "nothing happens".
#[test]
fn every_lint_pass_is_registered_with_the_lint_store() {
    let registered_passes = parse_registered_pass_structs(&lib_rs());
    assert!(
        !registered_passes.is_empty(),
        "parsed zero `register_late_pass` calls — the parser or the marker moved"
    );

    let passes = parse_pass_declarations();
    let lint_passes: Vec<&String> = passes
        .iter()
        .filter(|(_, lints)| !lints.is_empty())
        .map(|(name, _)| name)
        .collect();
    assert!(
        !lint_passes.is_empty(),
        "found no pass that drives a lint — the parser or the macro moved"
    );

    let unregistered: Vec<&&String> = lint_passes
        .iter()
        .filter(|name| !registered_passes.contains(**name))
        .collect();
    assert!(
        unregistered.is_empty(),
        "pass(es) driving a lint but never handed to the lint store: {:?}. Add a \
         `lint_store.register_late_pass(|_| Box::new(Pass))` call in \
         soroban_cost_lints/src/lib.rs; without it the lint never fires.",
        unregistered
    );
}

/// The guide has to name the registration points that exist in the code.
#[test]
fn developing_lints_guide_names_the_real_registration_points() {
    let guide = developing_lints_md();
    for needle in [
        "declare_lint!",
        "lint_store.register_lints(&[",
        "lint_store.register_late_pass(",
        "LINT_METADATA",
        "dylint_lint_impl!",
        "mod <module_name>;",
    ] {
        assert!(
            guide.contains(needle),
            "DEVELOPING_LINTS.md no longer names the `{needle}` registration point"
        );
    }
}

/// The guide must not claim that the `dylint_lint_impl!` list is what loads a
/// pass. That claim is what made the guide wrong in the first place.
#[test]
fn developing_lints_guide_does_not_claim_the_macro_loads_passes() {
    let guide = developing_lints_md();
    for forbidden in [
        "Dylint only loads passes listed below",
        "the classic cause of a lint that \"should work but never fires\"",
        "Dylint only loads passes",
    ] {
        assert!(
            !guide.contains(forbidden),
            "DEVELOPING_LINTS.md still claims the `dylint_lint_impl!` list loads passes: \
             found {forbidden:?}"
        );
    }
}
