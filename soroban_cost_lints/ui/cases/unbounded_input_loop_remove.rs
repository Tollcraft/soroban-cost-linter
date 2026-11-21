//! #1035: `remove` is a metered write.
//!
//! A `for` over a caller-supplied `soroban_sdk::Vec` whose only statement is
//! `storage.persistent().remove(&k)` has an attacker-chosen iteration count
//! and one host write per iteration, so it is the same cost shape as the
//! `set` loops — but it only gets reported once `remove` is in the write
//! table. The constant-bound pair proves the report is still conditional.

#![allow(
    soroban_storage_in_loop,
    loop_invariant_storage_access,
    storage_write_without_read,
    instance_storage_write_in_loop,
    persistent_read_without_ttl_extension,
    persistent_write_without_ttl_extension,
    linear_scan_in_loop,
    vec_where_slice_could_be_used,
    discarded_storage_read
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
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn remove<K>(&self, _k: &K) {}
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> {
                None
            }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn remove<K>(&self, _k: &K) {}
        }
    }

    pub mod vec {
        pub struct Vec<T>(std::vec::Vec<T>);
        impl<T> Vec<T> {
            pub fn iter(&self) -> std::slice::Iter<'_, T> {
                self.0.iter()
            }
            pub fn len(&self) -> usize {
                self.0.len()
            }
        }
    }

    pub use self::vec::Vec;
}

use soroban_sdk::{Env, Vec};

// --- Positive (should warn): remove is a write ---

fn bad_remove_over_input(env: Env, items: Vec<u32>) {
    for item in items.iter() {
        env.storage().persistent().remove(&item); //~ WARNING loop is driven by caller-supplied input and performs storage writes inside the loop body
    }
}

/// `.len()` of a caller-supplied collection is still a caller-chosen bound,
/// so the range form reports too.
fn bad_remove_over_input_len(env: Env, items: Vec<u32>) {
    for i in 0..items.len() {
        env.storage().instance().remove(&i); //~ WARNING loop is driven by caller-supplied input and performs storage writes inside the loop body
    }
}

// --- Negative (should not warn): contract-controlled bound ---

fn good_remove_constant_bound(env: Env) {
    for i in 0..10 {
        env.storage().persistent().remove(&i);
    }
}

/// Reads are not writes: `get` on the same caller-driven loop stays silent.
fn good_get_over_input(env: Env, items: Vec<u32>) {
    for item in items.iter() {
        let _: Option<u32> = env.storage().persistent().get(&item);
    }
}

fn main() {}
