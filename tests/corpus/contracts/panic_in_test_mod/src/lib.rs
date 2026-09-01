#![no_std]

#[cfg(test)]
mod tests {
    // This fixture exists so the linter can prove it does *not* flag a panic
    // inside `#[cfg(test)]`. The panic is the subject under test, so it is
    // expected when the fixture is compiled as a workspace member.
    #[test]
    #[should_panic(expected = "this should not trigger linter")]
    fn test_panic() {
        panic!("this should not trigger linter: {}", 1);
    }
}
