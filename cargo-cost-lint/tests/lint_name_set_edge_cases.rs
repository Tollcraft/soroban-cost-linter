//! Edge-case unit tests for `LintNameSet` (issue #1102).
//!
//! `LintNameSet` wraps a `HashSet<&'static str>` for O(1) lint-name lookups.
//! These tests cover empty strings, whitespace variants, case sensitivity, and
//! set-invariant assertions that the basic inline tests in `lint_name_set.rs`
//! do not exercise.

#[path = "../src/lint_name_set.rs"]
mod lint_name_set;

use lint_name_set::build_lint_name_set;

#[test]
fn empty_string_is_not_a_valid_lint_name() {
    let names: &'static [&str] = &["foo", "bar"];
    let set = build_lint_name_set(names);
    assert!(
        !set.contains(""),
        "empty string must not match any lint name"
    );
}

#[test]
fn lookup_is_case_sensitive_uppercase_does_not_match_lowercase() {
    let names: &'static [&str] = &["soroban_storage_in_loop"];
    let set = build_lint_name_set(names);
    assert!(set.contains("soroban_storage_in_loop"));
    assert!(!set.contains("SOROBAN_STORAGE_IN_LOOP"));
    assert!(!set.contains("Soroban_Storage_In_Loop"));
}

#[test]
fn lookup_is_case_sensitive_mixed_case_stored() {
    let names: &'static [&str] = &["MyLint", "anotherLint"];
    let set = build_lint_name_set(names);
    assert!(set.contains("MyLint"));
    assert!(set.contains("anotherLint"));
    assert!(!set.contains("mylint"));
    assert!(!set.contains("MYLINT"));
    assert!(!set.contains("ANOTHERLINT"));
}

#[test]
fn leading_whitespace_does_not_match() {
    let names: &'static [&str] = &["foo"];
    let set = build_lint_name_set(names);
    assert!(!set.contains(" foo"), "leading space must not match");
    assert!(!set.contains("\tfoo"), "leading tab must not match");
}

#[test]
fn trailing_whitespace_does_not_match() {
    let names: &'static [&str] = &["bar"];
    let set = build_lint_name_set(names);
    assert!(!set.contains("bar "), "trailing space must not match");
    assert!(!set.contains("bar\n"), "trailing newline must not match");
}

#[test]
fn embedded_whitespace_does_not_match_name_without_whitespace() {
    let names: &'static [&str] = &["foo_bar"];
    let set = build_lint_name_set(names);
    assert!(!set.contains("foo bar"));
    assert!(!set.contains("foo\tbar"));
}

#[test]
fn set_built_from_empty_slice_is_empty() {
    let names: &'static [&str] = &[];
    let set = build_lint_name_set(names);
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
    assert!(!set.contains("anything"));
    assert!(!set.contains(""));
}

#[test]
fn len_reflects_unique_entries_only() {
    let names: &'static [&str] = &["a", "a", "b", "b", "b"];
    let set = build_lint_name_set(names);
    assert_eq!(set.len(), 2, "duplicates must be collapsed");
}

#[test]
fn is_empty_false_for_non_empty_set() {
    let names: &'static [&str] = &["lint_one"];
    let set = build_lint_name_set(names);
    assert!(!set.is_empty());
}

#[test]
fn special_characters_in_name_match_exactly() {
    let names: &'static [&str] = &["lint-with-dashes", "lint::scoped"];
    let set = build_lint_name_set(names);
    assert!(set.contains("lint-with-dashes"));
    assert!(!set.contains("lint_with_dashes"));
    assert!(set.contains("lint::scoped"));
    assert!(!set.contains("lint_scoped"));
}

#[test]
fn lookup_on_large_set_still_returns_correct_results() {
    static MANY: &[&str] = &[
        "l00", "l01", "l02", "l03", "l04", "l05", "l06", "l07", "l08", "l09", "l10", "l11", "l12",
        "l13", "l14", "l15", "l16", "l17", "l18", "l19",
    ];
    let set = build_lint_name_set(MANY);
    assert_eq!(set.len(), 20);
    for name in MANY {
        assert!(set.contains(name));
    }
    assert!(!set.contains("l20"));
    assert!(!set.contains(""));
}
