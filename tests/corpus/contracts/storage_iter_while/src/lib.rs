#![no_std]
use soroban_sdk::{symbol_short, Env, Symbol};

const VALUE: Symbol = symbol_short!("value");

// Triggers soroban_storage_in_loop: while loop with storage get
pub fn bad_while(env: Env, key: i32) -> i32 {
    let mut result = 0;
    let mut i = 0;
    while i < key {
        let val: i32 = env.storage().instance().get(&VALUE).unwrap_or(0);
        result += val;
        i += 1;
    }
    result
}

// Good: while loop with storage operation outside
pub fn good_while(env: Env, count: i32) {
    let mut i = 0;
    while i < count {
        i += 1;
    }
    env.storage().instance().set(&VALUE, &i);
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{contract, contractimpl};

    /// Minimal registered contract used only to obtain a contract id, because
    /// storage is only reachable inside a contract context.
    #[contract]
    struct TestContract;

    #[contractimpl]
    impl TestContract {
        pub fn noop(_env: Env) {}
    }

    fn in_contract_context<R>(f: impl FnOnce(&Env) -> R) -> R {
        let env = Env::default();
        let id = env.register(TestContract, ());
        env.as_contract(&id, || f(&env))
    }

    /// Missing-key path: with nothing stored, every iteration reads the
    /// `unwrap_or(0)` default, so the sum is zero regardless of the bound.
    #[test]
    fn bad_while_defaults_to_zero_without_stored_value() {
        in_contract_context(|env| assert_eq!(bad_while(env.clone(), 5), 0));
    }

    /// The read happens once per iteration, so the result is `value * key`.
    #[test]
    fn bad_while_sums_one_read_per_iteration() {
        in_contract_context(|env| {
            env.storage().instance().set(&VALUE, &7i32);
            assert_eq!(bad_while(env.clone(), 3), 21);
            assert_eq!(bad_while(env.clone(), 1), 7);
        });
    }

    /// Boundary bounds: a zero or negative `key` never enters the loop.
    #[test]
    fn bad_while_skips_loop_for_non_positive_key() {
        in_contract_context(|env| {
            env.storage().instance().set(&VALUE, &7i32);
            assert_eq!(bad_while(env.clone(), 0), 0);
            assert_eq!(bad_while(env.clone(), -4), 0);
        });
    }

    /// `bad_while` only reads: the stored value is left untouched.
    #[test]
    fn bad_while_does_not_modify_storage() {
        in_contract_context(|env| {
            env.storage().instance().set(&VALUE, &7i32);
            bad_while(env.clone(), 4);
            assert_eq!(env.storage().instance().get::<Symbol, i32>(&VALUE), Some(7));
        });
    }

    /// Wrong-type path: the loop reads `i32`, so a value stored under a
    /// different type traps instead of silently defaulting to zero.
    #[test]
    #[should_panic]
    fn bad_while_traps_on_wrong_stored_type() {
        in_contract_context(|env| {
            env.storage().instance().set(&VALUE, &7i128);
            bad_while(env.clone(), 1);
        });
    }

    /// The single post-loop write persists the final counter, which equals
    /// `count` for any positive bound.
    #[test]
    fn good_while_persists_final_counter() {
        in_contract_context(|env| {
            good_while(env.clone(), 6);
            assert_eq!(env.storage().instance().get::<Symbol, i32>(&VALUE), Some(6));
        });
    }

    /// Boundary bounds: the write still happens once, storing zero.
    #[test]
    fn good_while_writes_zero_for_non_positive_count() {
        in_contract_context(|env| {
            good_while(env.clone(), 0);
            assert_eq!(env.storage().instance().get::<Symbol, i32>(&VALUE), Some(0));
            good_while(env.clone(), -3);
            assert_eq!(env.storage().instance().get::<Symbol, i32>(&VALUE), Some(0));
        });
    }

    /// Both entry points share `VALUE`: the good case overwrites whatever the
    /// bad case's input left behind, and the bad case then reads it back.
    #[test]
    fn good_while_output_feeds_bad_while() {
        in_contract_context(|env| {
            good_while(env.clone(), 4);
            assert_eq!(bad_while(env.clone(), 2), 8);
        });
    }
}
