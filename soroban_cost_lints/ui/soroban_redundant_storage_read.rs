#![allow(
    storage_write_without_read,
    persistent_read_without_ttl_extension,
    instance_storage_for_unbounded_data,
    loop_invariant_storage_access,
    soroban_storage_in_loop,
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

// Fires: two reads of the same key with no intervening write. The second read is
// flagged as redundant.
fn redundant_read(env: Env) {
    let _: Option<i32> = env.storage().instance().get(&1);
    let _: Option<i32> = env.storage().instance().get(&1); // Should Warn
}

// Near-miss: a write between the reads clears the tracked key, so the second
// read is not redundant and must NOT be flagged.
fn read_write_read(env: Env) {
    let _: Option<i32> = env.storage().instance().get(&1);
    env.storage().instance().set(&1, &2);
    let _: Option<i32> = env.storage().instance().get(&1); // Should NOT warn
}

// Near-miss: the two reads use different keys, so the second is not redundant.
fn different_keys(env: Env) {
    let _: Option<i32> = env.storage().instance().get(&1); // Should NOT warn
    let _: Option<i32> = env.storage().instance().get(&2); // Should NOT warn
}

fn main() {}
