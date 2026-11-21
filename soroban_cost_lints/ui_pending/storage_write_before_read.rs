//! Supplementary fixture for `storage_write_without_read` that specifically
//! tests the ordering fix: a read that appears *after* a write for the same key
//! must NOT suppress the warning.
pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env { fn clone(&self) -> Self { Env } }
    impl Env {
        pub fn storage(&self) -> storage::Storage { storage::Storage }
    }
    pub mod storage {
        pub struct Storage;
        impl Storage {
            pub fn instance(&self) -> Instance { Instance }
        }
        pub struct Instance;
        impl Instance {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
        }
    }
}
use soroban_sdk::Env;

/// Write first, then read the same key: the write has no prior read and MUST
/// produce a warning even though a read follows later in the body.
fn write_then_read(env: Env) {
    env.storage().instance().set(&1, &2); // Should Warn
    let _ = env.storage().instance().get::<i32, i32>(&1);
}

/// Read first, then write: canonical safe usage — no warning.
fn read_then_write(env: Env) {
    let _ = env.storage().instance().get::<i32, i32>(&1);
    env.storage().instance().set(&1, &2); // Good
}

fn main() {}
