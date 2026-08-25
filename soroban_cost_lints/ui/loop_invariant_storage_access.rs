#![allow(
    soroban_storage_in_loop,
    soroban_redundant_storage_read,
    storage_write_without_read,
    persistent_read_without_ttl_extension,
    instance_storage_for_unbounded_data,
    unbounded_input_loop
)]

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
            pub fn temporary(&self) -> Temporary { Temporary }
        }

        pub struct Instance;
        impl Instance {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn extend_ttl<K>(&self, _k: &K, _ttl: &()) {}
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
        }
    }
}

use soroban_sdk::Env;
use soroban_sdk::storage::Instance;

// Fires: a storage access inside a loop whose operands do not depend on the
// loop. `env` is a constant receiver, so every call in the chain
// (`storage` / `instance` / `get`) is loop-invariant.
fn invariant_get(env: Env) {
    for _ in 0..10 {
        let _: Option<i32> = env.storage().instance().get(&1); // Should Warn
    }
}

// Near-miss: the storage receiver `s` IS the loop variable, so the access
// depends on loop state and must NOT be flagged even though it is inside a loop.
fn varying_receiver(env: Env, stores: Vec<Instance>) {
    for s in stores.iter() {
        let _: Option<i32> = s.get(&1); // Should NOT warn
    }
}

fn main() {}
