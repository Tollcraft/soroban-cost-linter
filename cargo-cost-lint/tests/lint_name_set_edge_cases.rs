#[path = "../src/lint_name_set.rs"]
mod lint_name_set;
use lint_name_set::build_lint_name_set;

// ── whitespace ────────────────────────────────────────────────────────────────

#[test]
fn lookup_does_not_trim_leading_whitespace() {
    let names: &'static [&str] = &["foo"];
    let set = build_lint_name_set(names);
    // A caller who passes " foo" (with a leading space) must not get a hit,
    // because the stored name is "foo" without whitespace.
    assert!(
        !set.contains(" foo"),
        "leading-space variant must not match"
    );
}

#[test]
fn lookup_does_not_trim_trailing_whitespace() {
    let names: &'static [&str] = &["foo"];
    let set = build_lint_name_set(names);
    assert!(
        !set.contains("foo "),
        "trailing-space variant must not match"
    );
}

#[test]
fn lookup_does_not_trim_internal_whitespace() {
    let names: &'static [&str] = &["foo_bar"];
    let set = build_lint_name_set(names);
    assert!(
        !set.contains("foo bar"),
        "space-separated variant must not match"
    );
    assert!(
        !set.contains("foo\tbar"),
        "tab-separated variant must not match"
    );
}

#[test]
fn name_stored_with_whitespace_is_found_exactly() {
    // Edge case: a name that itself contains whitespace is stored and
    // retrieved byte-for-byte — the set does not normalise on insert.
    let names: &'static [&str] = &[" padded "];
    let set = build_lint_name_set(names);
    assert!(set.contains(" padded "), "exact padded name must match");
    assert!(!set.contains("padded"), "trimmed version must not match");
}

// ── empty string ──────────────────────────────────────────────────────────────

#[test]
fn empty_string_is_not_found_in_non_empty_set() {
    let names: &'static [&str] = &["foo", "bar"];
    let set = build_lint_name_set(names);
    assert!(
        !set.contains(""),
        "empty string must not match a non-empty-string name"
    );
}

#[test]
fn empty_string_stored_as_a_name_is_found() {
    // If "" is explicitly in the list, it should be retrievable.
    let names: &'static [&str] = &[""];
    let set = build_lint_name_set(names);
    assert!(
        set.contains(""),
        "explicitly stored empty string must be found"
    );
    assert_eq!(set.len(), 1);
}

#[test]
fn empty_set_does_not_match_empty_string() {
    let names: &'static [&str] = &[];
    let set = build_lint_name_set(names);
    assert!(
        !set.contains(""),
        "empty set must not match anything, including the empty string"
    );
}

// ── case sensitivity ──────────────────────────────────────────────────────────

#[test]
fn lookup_is_case_sensitive_uppercase_stored() {
    let names: &'static [&str] = &["SOROBAN_STORAGE_IN_LOOP"];
    let set = build_lint_name_set(names);
    assert!(set.contains("SOROBAN_STORAGE_IN_LOOP"));
    assert!(
        !set.contains("soroban_storage_in_loop"),
        "lowercase must not match uppercase stored name"
    );
    assert!(
        !set.contains("Soroban_Storage_In_Loop"),
        "mixed case must not match"
    );
}

#[test]
fn lookup_is_case_sensitive_lowercase_stored() {
    let names: &'static [&str] = &["soroban_storage_in_loop"];
    let set = build_lint_name_set(names);
    assert!(set.contains("soroban_storage_in_loop"));
    assert!(
        !set.contains("SOROBAN_STORAGE_IN_LOOP"),
        "uppercase must not match lowercase stored name"
    );
}

#[test]
fn mixed_case_names_coexist_independently() {
    let names: &'static [&str] = &["Foo", "foo", "FOO"];
    let set = build_lint_name_set(names);
    // All three are distinct entries.
    assert_eq!(set.len(), 3);
    assert!(set.contains("Foo"));
    assert!(set.contains("foo"));
    assert!(set.contains("FOO"));
    assert!(!set.contains("fOO"));
}

// ── special characters ────────────────────────────────────────────────────────

#[test]
fn names_with_underscores_match_exactly() {
    let names: &'static [&str] = &["soroban_storage_in_loop"];
    let set = build_lint_name_set(names);
    assert!(set.contains("soroban_storage_in_loop"));
    assert!(
        !set.contains("sorobanstoraginloop"),
        "without underscores must not match"
    );
    assert!(
        !set.contains("soroban-storage-in-loop"),
        "hyphen variant must not match"
    );
}

#[test]
fn names_with_hyphens_are_stored_exactly() {
    let names: &'static [&str] = &["some-lint-name"];
    let set = build_lint_name_set(names);
    assert!(set.contains("some-lint-name"));
    assert!(
        !set.contains("some_lint_name"),
        "underscore variant must not match hyphenated name"
    );
}

#[test]
fn unicode_names_are_matched_by_exact_code_point() {
    let names: &'static [&str] = &["lint\u{00e9}name"]; // "linté name" with combining accent as precomposed U+00E9
    let set = build_lint_name_set(names);
    assert!(
        set.contains("lint\u{00e9}name"),
        "exact Unicode name must match"
    );
    // Decomposed form (e + combining accent U+0301) is a different byte sequence.
    assert!(
        !set.contains("lint\u{0065}\u{0301}name"),
        "decomposed Unicode must not match precomposed"
    );
}

// ── set invariants ────────────────────────────────────────────────────────────

#[test]
fn len_reflects_unique_entries_only() {
    let names: &'static [&str] = &["a", "b", "a", "c", "b"];
    let set = build_lint_name_set(names);
    // Five entries in the slice, but only three unique values.
    assert_eq!(set.len(), 3);
}

#[test]
fn is_empty_only_when_all_entries_are_duplicate_of_one_another() {
    let names: &'static [&str] = &["x", "x", "x"];
    let set = build_lint_name_set(names);
    assert!(
        !set.is_empty(),
        "a set built from a non-empty slice is not empty even if all names duplicate"
    );
    assert_eq!(set.len(), 1);
}

#[test]
fn large_static_set_len_matches_unique_count() {
    static MANY_NAMES: &[&str] = &[
        "lint_a", "lint_b", "lint_c", "lint_d", "lint_e", "lint_f", "lint_g", "lint_h", "lint_i",
        "lint_j", "lint_a", // duplicate
    ];
    let set = build_lint_name_set(MANY_NAMES);
    assert_eq!(set.len(), 10, "one duplicate should not inflate the count");
    assert!(set.contains("lint_j"));
    assert!(!set.contains("lint_k"));
}
