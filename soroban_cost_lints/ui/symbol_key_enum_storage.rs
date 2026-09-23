#![allow(soroban_storage_in_loop, storage_write_without_read, redundant_env_clone)]

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
            pub fn instance(&self) -> Instance { Instance }
        }

        pub struct Instance;
        impl Instance {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
        }
    }

    pub trait ContractError {}
}

use soroban_sdk::{storage::Storage, Env};

// =======================================================================
// symbol_key_enum_storage — Fixtures
// =======================================================================

#[derive(Debug)]
enum StorageKey {
    Balance,
    Allowance,
    Config,
}

// --- Positive (should warn): discourages combining a type boundary
//     symbol with per-discriminant storage keys ---

fn bad_storage_key_store_enum(env: Env) {
    let key = StorageKey::Balance;
    env.storage().instance().set(&key, &1_u32);
}

// --- Negative (should not warn): scalar string keys stay as symbols ---

fn good_storage_key_plain_symbol(env: Env) {
    env.storage().instance().set(&"balance", &1_u32);
}

fn main() {}