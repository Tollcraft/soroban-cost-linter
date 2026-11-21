#![allow(
    soroban_storage_in_loop,
    loop_invariant_storage_access,
    storage_write_without_read,
    instance_storage_write_in_loop,
    vec_where_slice_could_be_used,
    discarded_storage_read
)]

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

    pub mod vec {
        /// Stand-in for `soroban_sdk::vec::Vec`: only a `len()` that resolves is needed.
        pub struct Vec<T>(core::marker::PhantomData<T>);
        impl<T> Vec<T> {
            pub fn len(&self) -> u32 { 0 }
        }
    }
}

use soroban_sdk::{vec::Vec, Env};

// Input-driven range (`0..input.len()`) whose body only reads: silent because
// the loop body performs no metered write.
fn good_len_range_read_only(env: Env, input: Vec<u32>) {
    for i in 0..input.len() {
        let _: Option<u32> = env.storage().instance().get(&i); // Good — read-only body
    }
}

// The write happens after the loop, so the loop body itself is write-free.
fn good_len_range_write_after_loop(env: Env, input: Vec<u32>) {
    for i in 0..input.len() {
        let _ = i;
    }
    env.storage().instance().set(&0u32, &1u32); // Good — write is outside the loop
}

// Positive control: same len-bounded range with a write inside the body.
fn bad_len_range_write_in_body(env: Env, input: Vec<u32>) {
    for i in 0..input.len() {
        env.storage().instance().set(&i, &i); // Should Warn
    }
}

fn main() {}
