//! This module serves as a negative control corpus for the Soroban cost linter.
//!
//! Specifically, it ensures that the linter does not flag `panic!` calls
//! that are contained within a `#[cfg(test)]` module, because panics are
//! expected and valid in test contexts.

#![no_std]

/// Test module containing a panic to verify linter behavior.
#[cfg(test)]
mod tests {
    /// This fixture exists so the linter can prove it does *not* flag a panic
    /// inside `#[cfg(test)]`. The panic is the subject under test, so it is
    /// expected when the fixture is compiled as a workspace member.
    ///
    /// The `#[should_panic]` attribute indicates that this test is meant to fail
    /// during normal execution, thus making the panic legitimate.
    #[test]
    #[should_panic(expected = "this should not trigger linter")]
    fn test_panic() {
        // We use string interpolation here to prevent simple regex-based suppression
        // from masking the real panic syntax tree.
        panic!("this should not trigger linter: {}", 1);
    }
}
