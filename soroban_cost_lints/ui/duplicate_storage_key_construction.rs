#![allow(soroban_storage_in_loop, storage_write_without_read, redundant_env_clone, symbol_new_for_short_literal)]

pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env {
        fn clone(&self) -> Self { Env }
    }
    impl Env {
        pub fn storage(&self) -> storage::Storage {
            storage::Storage
        }
    }

    pub mod storage {
        pub struct Storage;
        impl Storage {
            pub fn instance(&self) -> Instance { Instance }
            pub fn persistent(&self) -> Persistent { Persistent }
        }

        pub struct Instance;
        impl Instance {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
        }
    }

    pub struct Symbol;
    impl Symbol {
        pub fn new(_env: &Env, _s: &str) -> Symbol { Symbol }
    }

    pub struct Address;
}

use soroban_sdk::{Env, Symbol};

// =======================================================================
// duplicate_storage_key_construction — Fixtures
// =======================================================================

// --- Positive (should warn): same key constructed in two functions ---

fn get_balance(env: &Env) -> i128 {
    let key = Symbol::new(env, "balance"); // Should Warn — same key as set_balance
    let val: Option<i128> = env.storage().instance().get(&key);
    val.unwrap_or(0)
}

fn set_balance(env: &Env, amount: i128) {
    let key = Symbol::new(env, "balance"); // Should Warn — same key as get_balance
    env.storage().instance().set(&key, &amount);
}

// --- Three functions using same key ---

fn read_config(env: &Env) -> i32 {
    let key = Symbol::new(env, "config"); // Should Warn — same key as write_config and delete_config
    let val: Option<i32> = env.storage().persistent().get(&key);
    val.unwrap_or(0)
}

fn write_config(env: &Env, value: i32) {
    let key = Symbol::new(env, "config"); // Should Warn — same key as read_config and delete_config
    env.storage().persistent().set(&key, &value);
}

fn delete_config(env: &Env) {
    let key = Symbol::new(env, "config"); // Should Warn — same key as read_config and write_config
    env.storage().persistent().set(&key, &0);
}

// --- Negative (should not warn): different keys ---

fn get_balance_v2(env: &Env) -> i128 {
    let key = Symbol::new(env, "balance_v2"); // Good — different key
    let val: Option<i128> = env.storage().instance().get(&key);
    val.unwrap_or(0)
}

// --- Negative (should not warn): key constructed once via constant ---

const BALANCE_KEY: &str = "balance";

fn get_balance_const(env: &Env) -> i128 {
    let key = Symbol::new(env, BALANCE_KEY); // Good — referenced through constant
    let val: Option<i128> = env.storage().instance().get(&key);
    val.unwrap_or(0)
}

fn set_balance_const(env: &Env, amount: i128) {
    let key = Symbol::new(env, BALANCE_KEY); // Good — referenced through constant
    env.storage().instance().set(&key, &amount);
}

// --- Negative (should not warn): key construction differs (different literal) ---

fn get_item_a(env: &Env) -> i32 {
    let key = Symbol::new(env, "item_a"); // Good — different literal than get_item_b
    let val: Option<i32> = env.storage().instance().get(&key);
    val.unwrap_or(0)
}

fn get_item_b(env: &Env) -> i32 {
    let key = Symbol::new(env, "item_b"); // Good — different literal than get_item_a
    let val: Option<i32> = env.storage().instance().get(&key);
    val.unwrap_or(0)
}

// --- Negative (should not warn): key built from runtime-derived payload ---

fn get_dynamic(env: &Env, name: &str) -> i32 {
    let key = Symbol::new(env, name); // Good — runtime-derived payload
    let val: Option<i32> = env.storage().instance().get(&key);
    val.unwrap_or(0)
}

fn set_dynamic(env: &Env, name: &str, value: i32) {
    let key = Symbol::new(env, name); // Good — runtime-derived payload
    env.storage().instance().set(&key, &value);
}

// --- Suppression test ---

#[allow(duplicate_storage_key_construction)]
fn allowed_dup_key_1(env: &Env) {
    let key = Symbol::new(env, "suppressed"); // Good (allowed)
    env.storage().instance().set(&key, &1);
}

#[allow(duplicate_storage_key_construction)]
fn allowed_dup_key_2(env: &Env) {
    let key = Symbol::new(env, "suppressed"); // Good (allowed)
    let _val: Option<i32> = env.storage().instance().get(&key);
}

fn main() {}
