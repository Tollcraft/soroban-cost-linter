#![allow(
    soroban_storage_in_loop,
    soroban_redundant_storage_read,
    storage_write_without_read,
    persistent_read_without_ttl_extension,
    instance_storage_for_unbounded_data,
    unbounded_input_loop,
    instance_storage_write_in_loop,
    nested_loop_storage_access
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
            pub fn get<K, V>(&self, _key: &K) -> Option<V> {
                None
            }
            pub fn set<K, V>(&self, _key: &K, _value: &V) {}
            pub fn has<K>(&self, _key: &K) -> bool {
                false
            }
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _key: &K) -> Option<V> {
                None
            }
            pub fn set<K, V>(&self, _key: &K, _value: &V) {}
            pub fn has<K>(&self, _key: &K) -> bool {
                false
            }
            pub fn extend_ttl<K>(&self, _key: &K, _ttl: &()) {}
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _key: &K) -> Option<V> {
                None
            }
            pub fn set<K, V>(&self, _key: &K, _value: &V) {}
            pub fn has<K>(&self, _key: &K) -> bool {
                false
            }
        }
    }
}

use soroban_sdk::Env;

fn varying_value_write(env: Env, vals: Vec<u32>) {
    let storage = env.storage();
    for v in vals.iter() {
        // Exercise the standalone accessors as well as the entire write chain.
        let _ = env.storage();
        let _ = storage.instance();
        env.storage().instance().set(&1, v);
    }
}

fn literal_value_write(env: Env) {
    let storage = env.storage();
    for _ in 0..3 {
        let _ = env.storage();
        let _ = storage.instance();
        env.storage().instance().set(&1, &2);
    }
}

fn write_outside_loop_is_silent(env: Env) {
    env.storage().instance().set(&1, &2);
}

fn main() {}
