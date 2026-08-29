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
        }
    }

    pub struct Symbol;
    impl Symbol {
        pub fn new(_env: &Env, _s: &str) -> Symbol { Symbol }
    }
}

use soroban_sdk::{Env, Symbol};

// =======================================================================
// symbol_new_for_short_literal — Fixtures
// =======================================================================

// --- Positive (should warn): short literal <= 9 chars, valid chars ---

fn bad_symbol_new_short_literal(env: Env) {
    let _sym = Symbol::new(&env, "hello"); // Should Warn — 5 chars
}

fn bad_symbol_new_exactly_9_chars(env: Env) {
    let _sym = Symbol::new(&env, "abcdefghi"); // Should Warn — exactly 9 chars
}

fn bad_symbol_new_1_char(env: Env) {
    let _sym = Symbol::new(&env, "x"); // Should Warn — 1 char
}

fn bad_symbol_new_with_underscore(env: Env) {
    let _sym = Symbol::new(&env, "hello_wor"); // Should Warn — 9 chars with underscore
}

// --- Negative (should not warn): boundary and invalid cases ---

fn good_symbol_new_10_chars(env: Env) {
    let _sym = Symbol::new(&env, "abcdefghij"); // Good — 10 chars > 9
}

fn good_symbol_new_11_chars(env: Env) {
    let _sym = Symbol::new(&env, "hello_world"); // Good — 11 chars > 9
}

fn good_symbol_new_invalid_chars_hyphen(env: Env) {
    let _sym = Symbol::new(&env, "hello-world"); // Good — contains invalid char '-'
}

fn good_symbol_new_invalid_chars_space(env: Env) {
    let _sym = Symbol::new(&env, "hello world"); // Good — contains space
}

fn good_symbol_new_non_literal(env: Env) {
    let s = "hello";
    let _sym = Symbol::new(&env, s); // Good — not a string literal
}

fn good_symbol_new_empty(env: Env) {
    let _sym = Symbol::new(&env, ""); // Good — empty string
}

// --- Suppression test ---

#[allow(symbol_new_for_short_literal)]
fn allowed_symbol_new_short_literal(env: Env) {
    let _sym = Symbol::new(&env, "hello"); // Good (allowed)
}

fn main() {}
