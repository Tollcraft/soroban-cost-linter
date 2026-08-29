#![allow(
    soroban_redundant_storage_read,
    storage_write_without_read,
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

// Fires: a persistent read with no `extend_ttl` call in the function.
fn read_without_extend(env: Env) {
    let _: Option<i32> = env.storage().persistent().get(&1); // Should Warn
}

// Near-miss: the TTL is extended after the read, so the lint stays silent.
fn read_with_extend(env: Env) {
    let _: Option<i32> = env.storage().persistent().get(&1);
    env.storage().persistent().extend_ttl(&1, &()); // Should NOT warn
}

// Near-miss: reads of non-persistent storage are out of scope.
fn non_persistent_read(env: Env) {
    let _: Option<i32> = env.storage().instance().get(&1); // Should NOT warn
}

fn main() {}
