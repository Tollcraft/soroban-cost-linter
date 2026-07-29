use std::collections::HashSet;

/// O(1) lookup set for lint names, replacing the linear `.contains()` scan
/// on the static `LINT_NAMES` slice.
pub struct LintNameSet {
    inner: HashSet<&'static str>,
}

impl LintNameSet {
    pub fn contains(&self, name: &str) -> bool {
        self.inner.contains(name)
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

/// Build a `LintNameSet` from a static `&[&str]` slice.
///
/// Converts the slice into a `HashSet` so that `contains` checks are O(1)
/// instead of O(n).
pub fn build_lint_name_set(names: &'static [&str]) -> LintNameSet {
    let inner = names.iter().copied().collect();
    LintNameSet { inner }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_existing_name() {
        let names: &'static [&str] = &["foo", "bar", "baz"];
        let set = build_lint_name_set(names);
        assert!(set.contains("foo"));
        assert!(set.contains("bar"));
        assert!(set.contains("baz"));
    }

    #[test]
    fn does_not_contain_missing_name() {
        let names: &'static [&str] = &["foo", "bar"];
        let set = build_lint_name_set(names);
        assert!(!set.contains("qux"));
        assert!(!set.contains(""));
    }

    #[test]
    fn len_matches_input() {
        let names: &'static [&str] = &["a", "b", "c", "d"];
        let set = build_lint_name_set(names);
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn empty_slice_produces_empty_set() {
        let names: &'static [&str] = &[];
        let set = build_lint_name_set(names);
        assert!(set.is_empty());
        assert!(!set.contains("anything"));
    }

    #[test]
    fn duplicates_are_deduplicated() {
        let names: &'static [&str] = &["x", "x", "x"];
        let set = build_lint_name_set(names);
        assert_eq!(set.len(), 1);
        assert!(set.contains("x"));
    }

    #[test]
    fn case_sensitive_lookup() {
        let names: &'static [&str] = &["Foo", "BAR"];
        let set = build_lint_name_set(names);
        assert!(set.contains("Foo"));
        assert!(set.contains("BAR"));
        assert!(!set.contains("foo"));
        assert!(!set.contains("bar"));
    }
}
