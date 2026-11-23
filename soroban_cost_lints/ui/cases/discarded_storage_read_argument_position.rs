#![allow(
    storage_write_without_read,
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
        }
    }
}

use soroban_sdk::Env;

fn consume<T>(_v: T) {}

fn read_as_call_argument(env: Env) {
    consume(env.storage().instance().get::<i32, i32>(&1));
}

fn read_as_is_some_receiver(env: Env) {
    let _ = env.storage().instance().get::<i32, i32>(&1).is_some();
}

fn bare_discarded_read(env: Env) {
    env.storage().instance().get::<i32, i32>(&1);
}

fn main() {}
