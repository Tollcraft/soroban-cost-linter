#![allow(symbol_new_for_short_literal, soroban_storage_in_loop, loop_invariant_storage_access, storage_write_without_read, formatted_panic_payload)]

pub mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn storage(&self) -> storage::Storage {
            storage::Storage
        }
    }

    pub mod storage {
        pub struct Storage;
        impl Storage {
            pub fn instance(&self) -> Instance { Instance }
        }

        pub struct Instance;
        impl Instance {
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
        }
    }

    pub struct Symbol;
    impl Symbol {
        pub fn new(_env: &Env, _s: &str) -> Symbol { Symbol }
    }
}

use soroban_sdk::{Env, Symbol};

// =======================================================================
// storage_key_construction_in_loop — Fixtures
// =======================================================================

// --- Positive (should warn): invariant key constructed inside loop ---

fn bad_invariant_key_in_for_loop(env: Env) {
    for i in 0..10 {
        let key = Symbol::new(&env, "my_key"); // Should Warn — same key every iteration
        env.storage().instance().set(&key, &i);
    }
}

fn bad_invariant_key_in_while_loop(env: Env) {
    let mut i = 0;
    while i < 10 {
        let key = Symbol::new(&env, "counter"); // Should Warn — same key every iteration
        env.storage().instance().set(&key, &i);
        i += 1;
    }
}

fn bad_invariant_key_in_loop_loop(env: Env) {
    let mut count = 0;
    loop {
        let key = Symbol::new(&env, "fixed_key"); // Should Warn — same key every iteration
        env.storage().instance().set(&key, &count);
        count += 1;
        if count >= 5 {
            break;
        }
    }
}

// --- Negative (should not warn): iteration-dependent key ---

fn good_variant_key_in_for_loop(env: Env) {
    for i in 0..10 {
        let key = Symbol::new(&env, &format!("key_{}", i)); // Good — key depends on loop variable
        env.storage().instance().set(&key, &i);
    }
}

fn good_key_construction_outside_loop(env: Env) {
    let key = Symbol::new(&env, "my_key"); // Good — constructed once outside loop
    for i in 0..10 {
        env.storage().instance().set(&key, &i);
    }
}

// --- Suppression test ---

#[allow(storage_key_construction_in_loop)]
fn allowed_key_construction_in_loop(env: Env) {
    for i in 0..10 {
        let key = Symbol::new(&env, "my_key"); // Good (allowed)
        env.storage().instance().set(&key, &i);
    }
}

fn main() {}
