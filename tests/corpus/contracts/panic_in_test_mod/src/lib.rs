#![no_std]

#[cfg(test)]
mod tests {
    #[test]
    fn test_panic() {
        panic!("this should not trigger linter: {}", 1);
    }
}