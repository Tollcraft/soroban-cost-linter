#![allow(
    soroban_storage_in_loop,
    instance_storage_write_in_loop,
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
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
        }
    }
}

use soroban_sdk::Env;

fn for_body_write(env: Env) {
    for i in 0..10 {
        env.storage().instance().set(&i, &2);
    }
}

fn if_nested_write(env: Env) {
    for i in 0..10 {
        if i > 5 {
            env.storage().instance().set(&i, &2);
        }
    }
}

fn read_then_write_in_loop(env: Env) {
    for i in 0..10 {
        let _ = env.storage().instance().get::<i32, i32>(&i);
        env.storage().instance().set(&i, &2);
    }
}

fn main() {}
