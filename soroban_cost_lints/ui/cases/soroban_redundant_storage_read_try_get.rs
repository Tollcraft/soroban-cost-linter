#![allow(dead_code, unused_variables)]

mod soroban_sdk {
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
        }
    }
}

use soroban_sdk::Env;

// Exercises try_get(&k); set(&k, ..); get(&k)
fn try_get_set_get(env: Env) {
    let a: Option<i32> = env.storage().instance().try_get(&1);
    env.storage().instance().set(&1, &2);
    let b: Option<i32> = env.storage().instance().get(&1);
    let _ = (a, b);
}

fn main() {}
