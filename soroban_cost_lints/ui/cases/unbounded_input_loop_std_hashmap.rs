//! #1036: `std` collections are not caller-supplied input.
//!
//! The collection test used to scan the rendered type name for `Vec`, `Map`,
//! `Array`, `Bytes` and `Set`, so `HashMap<u32, u32>` — which contains `Map` —
//! looked caller-driven, and a `#[allow]` on that report silenced every other
//! collection in the function too. The test now matches Soroban definition
//! paths, so the four `std` loops below emit nothing while the Soroban `Map`
//! and `Vec` pairs each still emit exactly one warning.

#![allow(
    soroban_storage_in_loop,
    loop_invariant_storage_access,
    storage_write_without_read,
    instance_storage_write_in_loop,
    persistent_read_without_ttl_extension,
    persistent_write_without_ttl_extension,
    linear_scan_in_loop,
    map_iter_then_lookup,
    vec_where_slice_could_be_used
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
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
        }

        pub struct Persistent;
        impl Persistent {
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
        }
    }

    pub mod map {
        pub struct Map<K, V>(core::marker::PhantomData<(K, V)>);
        impl<K, V> Map<K, V> {
            pub fn iter(&self) -> std::vec::Vec<(K, V)> {
                std::vec::Vec::new()
            }
        }
    }

    pub mod vec {
        pub struct Vec<T>(std::vec::Vec<T>);
        impl<T> Vec<T> {
            pub fn iter(&self) -> std::slice::Iter<'_, T> {
                self.0.iter()
            }
        }
    }

    pub mod bytes {
        pub struct Bytes(std::vec::Vec<u8>);
        impl Bytes {
            pub fn iter(&self) -> std::slice::Iter<'_, u8> {
                self.0.iter()
            }
        }
    }

    pub use self::bytes::Bytes;
    pub use self::map::Map;
    pub use self::vec::Vec;
}

use soroban_sdk::{Bytes, Env, Map, Vec};
use std::collections::{BTreeMap, HashMap, HashSet};

// --- Negative (should not warn): contract-built `std` collections ---

fn good_hashmap_iter(env: Env) {
    let mut weights: HashMap<u32, u32> = HashMap::new();
    weights.insert(1, 2);
    for (k, v) in weights.iter() {
        env.storage().persistent().set(k, v);
    }
}

fn good_hashmap_borrow(env: Env) {
    let mut seen: HashSet<u32> = HashSet::new();
    seen.insert(1);
    for k in &seen {
        env.storage().instance().set(k, k);
    }
}

fn good_btreemap_keys(env: Env) {
    let mut ordered: BTreeMap<u32, u32> = BTreeMap::new();
    ordered.insert(1, 2);
    for k in ordered.keys() {
        env.storage().persistent().set(k, k);
    }
}

fn good_std_vec(env: Env) {
    let items = std::vec![1u32, 2, 3];
    for item in items.iter() {
        env.storage().instance().set(item, item);
    }
}

// --- Positive (should warn): the same shapes with Soroban types ---

fn bad_soroban_map_iter(env: Env, roles: Map<u32, u32>) {
    for (k, v) in roles.iter() {
        env.storage().persistent().set(&k, &v); //~ WARNING loop is driven by caller-supplied input and performs storage writes inside the loop body
    }
}

fn bad_soroban_vec_iter(env: Env, items: Vec<u32>) {
    for item in items.iter() {
        env.storage().instance().set(&item, &item); //~ WARNING loop is driven by caller-supplied input and performs storage writes inside the loop body
    }
}

/// `Bytes` is accepted by its own row: `soroban_sdk::bytes::Bytes`.
fn bad_soroban_bytes_iter(env: Env, payload: Bytes) {
    for byte in payload.iter() {
        env.storage().persistent().set(&byte, &byte); //~ WARNING loop is driven by caller-supplied input and performs storage writes inside the loop body
    }
}

fn main() {}
