#![allow(storage_write_without_read)]

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
            pub fn set<K: ?Sized, V>(&self, _k: &K, _v: &V) {}
        }
    }
}

use soroban_sdk::Env;

fn bad_ref_option_u32(env: Env) {
    let val: Option<u32> = Some(42);
    env.storage().instance().set(&"key", &val); // Should Warn
}

fn bad_double_ref_option_u32(env: Env) {
    let val: Option<u32> = Some(42);
    let r_val = &val;
    env.storage().instance().set(&"key", &r_val); // Should Warn
}

fn main() {}
