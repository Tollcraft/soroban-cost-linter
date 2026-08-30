#![allow(
    soroban_storage_in_loop,
    soroban_redundant_storage_read,
    storage_write_without_read,
    persistent_read_without_ttl_extension,
    instance_storage_for_unbounded_data,
    unbounded_input_loop,
    symbol_new_for_short_literal,
    formatted_panic_payload,
    redundant_env_clone
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

// Case 1: a key read but never written anywhere in the crate -> FIRES.
fn reads_missing_key(env: Env) {
    let _: Option<i32> = env.storage().persistent().get(&42);
}

// Case 2: the same key is written in a different function -> does NOT fire.
fn writes_elsewhere(env: Env) {
    env.storage().persistent().set(&7, &1);
}

fn reads_written_elsewhere(env: Env) {
    let _: Option<i32> = env.storage().persistent().get(&7);
}

// Case 3: a dynamic key (function parameter) -> does NOT fire, and must NOT
// suppress findings about other keys.
fn reads_dynamic_key(env: Env, key: u32) {
    let _: Option<i32> = env.storage().persistent().get(&key);
}

// Case 4: a dynamic read must not suppress a static read-never-written key in
// the same crate -> the static key still FIRES.
fn reads_static_and_dynamic(env: Env, key: u32) {
    let _: Option<i32> = env.storage().persistent().get(&99);
    let _: Option<i32> = env.storage().persistent().get(&key);
}

// Case 5: instance vs persistent are distinct key spaces; a persistent write
// does not satisfy an instance read of the same literal -> FIRES.
fn instance_read_with_persistent_write(env: Env) {
    env.storage().persistent().set(&5, &1);
    let _: Option<i32> = env.storage().instance().get(&5);
}

fn main() {}
