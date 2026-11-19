#![no_std]
use soroban_sdk::{symbol_short, Env, Symbol};

const MIXED: Symbol = symbol_short!("mixed");

// Triggers soroban_storage_in_loop: storage get inside a loop
pub fn bad_mixed_get(env: Env) {
    for _ in 0..10 {
        let _val: i32 = env.storage().instance().get(&MIXED).unwrap_or(0);
    }
}

// Triggers soroban_storage_in_loop: storage set inside a loop with different storage type
pub fn bad_mixed_persistent(env: Env) {
    for i in 0..10 {
        env.storage().persistent().set(&MIXED, &i);
    }
}

// Good: loop accumulates in memory, single storage write after
pub fn good_mixed(env: Env) {
    let mut total = 0i128;
    for i in 0..10 {
        total += i;
    }
    env.storage().instance().set(&MIXED, &total);
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_bad_mixed_get() {
        let env = Env::default();
        // Sets a value to ensure the get logic covers both paths if needed, 
        // although the unwrap_or(0) already makes it safe to run empty.
        env.storage().instance().set(&MIXED, &42i32);
        bad_mixed_get(env.clone());
        // The function doesn't return anything or modify state, just ensure it runs.
    }

    #[test]
    fn test_bad_mixed_persistent() {
        let env = Env::default();
        bad_mixed_persistent(env.clone());
        let val: i32 = env.storage().persistent().get(&MIXED).unwrap();
        assert_eq!(val, 9);
    }

    #[test]
    fn test_good_mixed() {
        let env = Env::default();
        good_mixed(env.clone());
        let val: i128 = env.storage().instance().get(&MIXED).unwrap();
        assert_eq!(val, 45); // sum of 0 to 9 is 45
    }
}