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
            pub fn extend_ttl<K>(&self, _k: &K, _threshold: u32, _extend_to: u32) {}
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

use soroban_sdk::Env;

// Should Warn: persistent write with no extend_ttl
fn bad_write_without_extend(env: Env) {
    env.storage().persistent().set(&1, &42);
}

// Should Not Warn: persistent write with extend_ttl on same key
fn good_write_with_extend(env: Env) {
    env.storage().persistent().set(&1, &42);
    env.storage().persistent().extend_ttl(&1, 1000, 10000);
}

// Should Not Warn: instance storage write
fn good_instance_write(env: Env) {
    env.storage().instance().set(&1, &42);
}

// Should Not Warn: #[allow(...)]
#[allow(persistent_write_without_ttl_extension)]
fn good_allowed(env: Env) {
    env.storage().persistent().set(&1, &42);
}

fn main() {}
