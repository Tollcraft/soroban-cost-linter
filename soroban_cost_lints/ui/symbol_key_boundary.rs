#![allow(soroban_storage_in_loop, storage_write_without_read, redundant_env_clone, symbol_new_for_short_literal)]

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
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
        }
    }

    pub struct Symbol;
    impl Symbol {
        pub fn new(_env: &Env, _s: &str) -> Symbol { Symbol }
    }
}

use soroban_sdk::{Env, Symbol};

// =======================================================================
// symbol_key_boundary — Fixtures
// =======================================================================

// --- Positive (should warn): symbol keys at or beyond the 32-byte boundary ---

fn bad_symbol_key_over_32_bytes(env: Env) {
    let _sym = Symbol::new(&env, "some_symbol_key_that_is_far_too_long_0001");
}

fn bad_symbol_key_near_32_bytes(env: Env) {
    let _sym = Symbol::new(&env, "symbol_key_that_hits_the_boundary_");
}

// --- Negative (should not warn): keys comfortably within the limit ---

fn good_short_symbol_key(env: Env) {
    let _sym = Symbol::new(&env, "valid");
}

fn good_long_symbol_key_capped(env: Env) {
    let _sym = Symbol::new(&env, "well_within_32_bytes");
}

fn main() {}