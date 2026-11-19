#![allow(unwrap_on_storage_get)]

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

#[derive(Clone)]
pub struct BigConfig {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

pub struct SingleField {
    pub a: u32,
}

// Should Warn: only `a` is accessed
fn bad_single_field(env: Env) -> u32 {
    let cfg: BigConfig = env.storage().instance().get(&1).unwrap();
    cfg.a
}

// Should Not Warn: multiple fields accessed
fn good_multiple_fields(env: Env) -> u32 {
    let cfg: BigConfig = env.storage().instance().get(&1).unwrap();
    cfg.a + cfg.b
}

// Should Not Warn: single field struct
fn good_single_field_struct(env: Env) -> u32 {
    let s: SingleField = env.storage().instance().get(&1).unwrap();
    s.a
}

// Should Not Warn: passed by reference to helper
fn helper(_cfg: &BigConfig) {}
fn good_passed_to_helper(env: Env) {
    let cfg: BigConfig = env.storage().instance().get(&1).unwrap();
    helper(&cfg);
}

// Should Not Warn: #[allow(...)]
#[allow(large_value_decoded_for_one_field)]
fn good_allowed(env: Env) -> u32 {
    let cfg: BigConfig = env.storage().instance().get(&1).unwrap();
    cfg.a
}

fn main() {}
