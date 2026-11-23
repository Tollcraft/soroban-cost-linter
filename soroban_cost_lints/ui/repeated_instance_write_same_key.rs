#![warn(repeated_instance_write_same_key)]
#![allow(
    storage_write_without_read,
    discarded_storage_read,
    dead_code,
    unused_variables
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
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> {
                None
            }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
        }
    }

    pub struct Symbol;
}

use soroban_sdk::{Env, Symbol};

fn bad_two_writes_local_key(env: &Env) {
    let key = Symbol;
    env.storage().instance().set(&key, &1);
    env.storage().instance().set(&key, &2); //~ WARNING repeated instance storage write to the same key
}

fn good_different_keys(env: &Env) {
    let key1 = Symbol;
    let key2 = Symbol;
    env.storage().instance().set(&key1, &1);
    env.storage().instance().set(&key2, &2);
}

fn good_fn_1(env: &Env) {
    let key = Symbol;
    env.storage().instance().set(&key, &1);
}

fn good_fn_2(env: &Env) {
    let key = Symbol;
    env.storage().instance().set(&key, &2);
}

fn good_interleaved_with_get(env: &Env) {
    let key = Symbol;
    env.storage().instance().set(&key, &1);
    let _val: Option<i32> = env.storage().instance().get(&key);
    env.storage().instance().set(&key, &2);
}

fn good_persistent_storage(env: &Env) {
    let key = Symbol;
    env.storage().persistent().set(&key, &1);
    env.storage().persistent().set(&key, &2);
}

fn good_temporary_storage(env: &Env) {
    let key = Symbol;
    env.storage().temporary().set(&key, &1);
    env.storage().temporary().set(&key, &2);
}

#[allow(repeated_instance_write_same_key)]
fn allowed_repeated_write(env: &Env) {
    let key = Symbol;
    env.storage().instance().set(&key, &1);
    env.storage().instance().set(&key, &2);
}

fn main() {}
