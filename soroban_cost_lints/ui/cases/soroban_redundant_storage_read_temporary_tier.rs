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
            pub fn temporary(&self) -> Temporary {
                Temporary
            }
        }

        pub struct Instance;
        impl Instance {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> {
                None
            }
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> {
                None
            }
        }
    }
}

use soroban_sdk::Env;

// Fires: two temporary().get(&k) calls on one key warn once, on the second.
fn redundant_read_temporary(env: Env) {
    let a1: Option<i32> = env.storage().temporary().get(&1);
    let a2: Option<i32> = env.storage().temporary().get(&1); // Should Warn
    let _ = (a1, a2);
}

// Near-miss: instance then temporary read of one key is silent.
fn instance_then_temporary_different_tiers(env: Env) {
    let a: Option<i32> = env.storage().instance().get(&1);
    let b: Option<i32> = env.storage().temporary().get(&1); // Should NOT warn
    let _ = (a, b);
}

fn main() {}
