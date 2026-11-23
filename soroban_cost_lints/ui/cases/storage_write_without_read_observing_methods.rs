#![allow(
    discarded_storage_read,
    dead_code,
    unused_variables
)]

pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env {
        fn clone(&self) -> Self {
            Env
        }
    }
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
            pub fn try_get<K, V>(&self, _k: &K) -> Option<V> {
                None
            }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool {
                false
            }
            pub fn remove<K>(&self, _k: &K) -> bool {
                false
            }
            pub fn update<K>(&self, _k: &K, _f: i32) -> bool {
                false
            }
        }
    }
}

use soroban_sdk::Env;

fn test_get_observes(env: Env) {
    let _ = env.storage().instance().get::<i32, i32>(&1);
    env.storage().instance().set(&1, &2);
}

fn test_try_get_observes(env: Env) {
    let _ = env.storage().instance().try_get::<i32, i32>(&1);
    env.storage().instance().set(&1, &2);
}

fn test_has_observes(env: Env) {
    let _ = env.storage().instance().has(&1);
    env.storage().instance().set(&1, &2);
}

fn test_remove_observes(env: Env) {
    let _ = env.storage().instance().remove(&1);
    env.storage().instance().set(&1, &2);
}

fn test_update_observes(env: Env) {
    let _ = env.storage().instance().update(&1, 42);
    env.storage().instance().set(&1, &2);
}

fn main() {}
