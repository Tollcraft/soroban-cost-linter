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
}

use soroban_sdk::Env;

// =======================================================================
// u128_where_u64_suffices — Fixtures
// =======================================================================

// --- Positive (should warn): u128 used where u64 suffices ---

fn bad_counter_arithmetic() {
    let mut i: u128 = 0;
    i = i + 1;
}

fn bad_derived_from_u32_len(len: u32) {
    let x: u128 = len as u128;
    let _y = x * 2;
}

// --- Negative (should not warn): genuine u128 token balances ---

fn good_token_balance(balance: u128, delta: u128) {
    let _new_balance = balance + delta;
}

fn good_already_u64(a: u64, b: u64) {
    let _c = a * b;
}

fn main() {}