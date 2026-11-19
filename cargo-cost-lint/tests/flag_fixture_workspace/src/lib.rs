//! Fixture workspace for the `--no-cache`, `--clear-cache`, `.lintignore`,
//! `--diff-only` and `--quiet` end-to-end tests.
//!
//! It has no dependencies, so the tests run without network access, and it
//! carries one finding per file so a filter can remove exactly one of them.
//! The finding is `blind_storage_write`: a key is read, written, and then
//! written again with no read in between.

pub mod other;

/// The minimal `soroban_sdk` storage surface `blind_storage_write` matches.
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
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
        }
    }
}

use soroban_sdk::Env;

/// `blind_storage_write` finding in `src/lib.rs`: the second write discards the
/// first store without reading it back.
pub fn lib_finding(env: Env) {
    let _ = env.storage().instance().get::<u32, u32>(&1);
    env.storage().instance().set(&1, &2);
    env.storage().instance().set(&1, &3);
}
