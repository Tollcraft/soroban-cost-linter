#![allow(
    soroban_storage_in_loop,
    soroban_redundant_storage_read,
    storage_write_without_read,
    persistent_read_without_ttl_extension,
    instance_storage_for_unbounded_data,
    unbounded_input_loop,
    instance_storage_write_in_loop,
    nested_loop_storage_access,
    discarded_storage_read
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
            pub fn persistent(&self) -> Persistent {
                Persistent
            }
            pub fn temporary(&self) -> Temporary {
                Temporary
            }
        }

        pub struct Instance;
        impl Instance {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> {
                None
            }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool {
                false
            }
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> {
                None
            }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool {
                false
            }
            pub fn extend_ttl<K>(&self, _k: &K, _ttl: &()) {}
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> {
                None
            }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool {
                false
            }
        }
    }
}

pub mod user {
    pub struct Persistent;
    impl Persistent {
        pub fn get<K, V>(&self, _k: &K) -> Option<V> {
            None
        }
        pub fn set<K, V>(&self, _k: &K, _v: &V) {}
        pub fn has<K>(&self, _k: &K) -> bool {
            false
        }
        pub fn extend_ttl<K>(&self, _k: &K, _ttl: &()) {}
    }
}

use soroban_sdk::Env;

fn lookalike_storage_in_loop(s: user::Persistent) {
    for _ in 0..10 {
        s.set(&1, &2);
        let _ = s.get::<i32, i32>(&1);
        let _ = s.has(&1);
        s.extend_ttl(&1, &());
    }
}

fn control_real_storage_in_loop(env: Env) {
    for _ in 0..10 {
        env.storage().persistent().set(&1, &2); // Should Warn
    }
}

fn main() {}
