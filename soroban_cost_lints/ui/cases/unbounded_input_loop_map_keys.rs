//! #1034: `Map::keys()` and `Map::values()` are caller-driven iteration.
//!
//! The mock declares the SDK modules by their real paths
//! (`soroban_sdk::map::Map`, `soroban_sdk::vec::Vec`) because the collection
//! test matches definition paths, not rendered names: a type declared at
//! `soroban_sdk::Map` is a fixture invention and matches nothing.
//!
//! The `std::collections::HashMap` case is the pair to #1036 — the same
//! `keys()` spelling on a map the contract built itself, which must stay
//! silent.

#![allow(
    soroban_storage_in_loop,
    loop_invariant_storage_access,
    storage_write_without_read,
    instance_storage_write_in_loop,
    persistent_read_without_ttl_extension,
    persistent_write_without_ttl_extension,
    linear_scan_in_loop,
    map_iter_then_lookup
)]

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
            pub fn persistent(&self) -> Persistent {
                Persistent
            }
        }

        pub struct Instance;
        impl Instance {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> {
                None
            }
            pub fn has<K>(&self, _k: &K) -> bool {
                false
            }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn remove<K>(&self, _k: &K) {}
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> {
                None
            }
            pub fn has<K>(&self, _k: &K) -> bool {
                false
            }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn remove<K>(&self, _k: &K) {}
        }
    }

    pub mod map {
        pub struct Map<K, V>(core::marker::PhantomData<(K, V)>);
        impl<K, V> Map<K, V> {
            // Returning `std::vec::Vec` keeps the file dependency-free; the
            // lint looks at the receiver, not at what the projection yields.
            pub fn keys(&self) -> std::vec::Vec<K> {
                std::vec::Vec::new()
            }
            pub fn values(&self) -> std::vec::Vec<V> {
                std::vec::Vec::new()
            }
            pub fn iter(&self) -> std::vec::Vec<(K, V)> {
                std::vec::Vec::new()
            }
        }
    }

    pub use self::map::Map;
}

use soroban_sdk::{Env, Map};
use std::collections::HashMap;

// --- Positive (should warn): keys() over a caller-supplied Map ---

fn bad_keys_loop_writes(env: Env, roles: Map<u32, u32>) {
    for k in roles.keys() {
        env.storage().persistent().set(&k, &k); //~ WARNING loop is driven by caller-supplied input and performs storage writes inside the loop body
    }
}

// --- Positive (should warn): values() over a caller-supplied Map ---

fn bad_values_loop_writes(env: Env, roles: Map<u32, u32>) {
    for v in roles.values() {
        env.storage().instance().set(&v, &v); //~ WARNING loop is driven by caller-supplied input and performs storage writes inside the loop body
    }
}

// --- Positive (should warn): iter() destructured over a caller-supplied Map ---

fn bad_iter_loop_writes(env: Env, roles: Map<u32, u32>) {
    for (k, v) in roles.iter() {
        env.storage().persistent().set(&k, &v); //~ WARNING loop is driven by caller-supplied input and performs storage writes inside the loop body
    }
}

// --- Negative (should not warn): caller-driven iteration, read-only body ---

fn good_keys_loop_only_reads(env: Env, roles: Map<u32, u32>) -> u32 {
    let mut present = 0u32;
    for k in roles.keys() {
        if env.storage().persistent().has(&k) {
            present += 1;
        }
    }
    present
}

// --- Negative (should not warn): a map the contract built itself ---

fn good_std_hashmap_keys(env: Env, budget: u32) {
    let mut weights: HashMap<u32, u32> = HashMap::new();
    weights.insert(1, budget);
    for k in weights.keys() {
        env.storage().persistent().set(k, k);
    }
}

fn main() {}
