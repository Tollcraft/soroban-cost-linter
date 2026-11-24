#![allow(soroban_storage_in_loop, dead_code, unused_variables)]

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

// Nested block: statement inside nested block warns
fn nested_block(env: Env) {
    {
        env.storage().instance().get::<u32, i32>(&1); // Should Warn
    }
}

// Closure body: statement in closure body warns
fn closure_body_statement(env: Env) {
    let f = || {
        env.storage().instance().get::<u32, i32>(&1); // Should Warn
    };
    f();
}

// Closure tail: read returned from closure is not discarded and stays silent
fn closure_tail_return(env: Env) -> Option<i32> {
    let f = || env.storage().instance().get::<u32, i32>(&1); // Should NOT warn
    f()
}

fn main() {}
