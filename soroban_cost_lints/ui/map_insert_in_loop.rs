#![allow(bytes_append_in_loop)]

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
            pub fn instance(&self) -> Instance {
                Instance
            }
        }

        pub struct Instance;
        impl Instance {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> {
                None
            }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
        }
    }

    pub struct Map;
    impl Map {
        pub fn new() -> Map {
            Map
        }
        pub fn set<K, V>(&mut self, _k: K, _v: V) {}
        pub fn get<K: ?Sized, V>(&self, _k: &K) -> Option<V> {
            None
        }
    }
}

use soroban_sdk::{Env, Map};

// =======================================================================
// map_insert_in_loop — Fixtures
// =======================================================================

// --- Positive (should warn): Map::set inside a loop ---

fn bad_map_insert_in_for_loop(env: Env) {
    let mut map = Map::new();
    for i in 0..10 {
        map.set(i, i * 2); // Should Warn
    }
    let _ = env;
}

fn bad_map_insert_in_while_loop(env: Env) {
    let mut map = Map::new();
    let mut i = 0;
    while i < 10 {
        map.set(i, i + 1); // Should Warn
        i += 1;
    }
    let _ = env;
}

fn bad_map_insert_in_loop_loop(env: Env) {
    let mut map = Map::new();
    let mut count = 0;
    loop {
        map.set(count, count); // Should Warn
        count += 1;
        if count >= 5 {
            break;
        }
    }
    let _ = env;
}

fn bad_map_set_mirroring_map_operations(env: Env, accounts: [i32; 3]) {
    let mut balances = Map::new();
    for account in accounts.iter() {
        balances.set(*account, 100); // Should Warn
    }
    let _ = env;
}

// --- Negative (should not warn): Map::set outside a loop ---

fn good_map_insert_outside_loop(env: Env) {
    let mut map = Map::new();
    map.set(1, 10); // Good — not in a loop
    map.set(2, 20); // Good — not in a loop
    let _ = env;
}

fn good_std_hashmap_inside_loop() {
    let mut map = std::collections::HashMap::new();
    for i in 0..10 {
        map.insert(i, i * 2); // Good — std::collections::HashMap
    }
}

// --- Suppression test ---

#[allow(map_insert_in_loop)]
fn allowed_map_insert_in_loop(env: Env) {
    let mut map = Map::new();
    for i in 0..10 {
        map.set(i, i); // Good (allowed)
    }
    let _ = env;
}

fn main() {}
