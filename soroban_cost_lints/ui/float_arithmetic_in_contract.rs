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

    pub struct Symbol;
    impl Symbol {
        pub fn new(_env: &Env, _s: &str) -> Symbol { Symbol }
    }
}

use soroban_sdk::Env;

// =======================================================================
// float_arithmetic_in_contract — Fixtures
// =======================================================================

// --- Positive (should warn): f32/f64 arithmetic inside contract code ---

fn bad_f64_price_calculation(env: Env) {
    let price: f64 = 1.5;
    let quantity: f64 = 10.0;
    let _total: f64 = price * quantity; // Should Warn — f64 multiply
}

fn bad_f32_balance(env: Env) {
    let balance: f32 = 100.0;
    let rate: f32 = 0.05;
    let _interest: f32 = balance * rate; // Should Warn — f32 multiply
}

fn bad_f64_division(env: Env) {
    let a: f64 = 100.0;
    let b: f64 = 3.0;
    let _result: f64 = a / b; // Should Warn — f64 division
}

fn bad_f64_addition(env: Env) {
    let a: f64 = 1.0;
    let b: f64 = 2.0;
    let _sum: f64 = a + b; // Should Warn — f64 addition
}

fn bad_f64_subtraction(env: Env) {
    let a: f64 = 10.0;
    let b: f64 = 3.0;
    let _diff: f64 = a - b; // Should Warn — f64 subtraction
}

// --- Negative (should not warn): integer arithmetic ---

fn good_integer_arithmetic(env: Env) {
    let price: i128 = 1500;
    let quantity: i128 = 10;
    let _total: i128 = price * quantity; // Good — integer arithmetic
}

// --- Negative (should not warn): integer division ---

fn good_integer_division(env: Env) {
    let a: i128 = 100;
    let b: i128 = 3;
    let _result: i128 = a / b; // Good — integer division
}

// --- Suppression test ---

#[allow(float_arithmetic_in_contract)]
fn allowed_f64_calculation(env: Env) {
    let a: f64 = 1.0;
    let b: f64 = 2.0;
    let _sum: f64 = a + b; // Good (allowed)
}

fn main() {}
