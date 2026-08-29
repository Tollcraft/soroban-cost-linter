#![allow(
    soroban_storage_in_loop,
    loop_invariant_storage_access,
    storage_write_without_read
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
            pub fn persistent(&self) -> Persistent { Persistent }
            pub fn temporary(&self) -> Temporary { Temporary }
        }

        pub struct Instance;
        impl Instance {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
        }
    }
}

use soroban_sdk::Env;

// =======================================================================
// unbounded_input_loop — Fixtures
// =======================================================================

// --- Positive (should warn): loop bound from caller input + storage write ---

fn bad_for_loop_bound_from_param(env: Env, count: u32) {
    for i in 0..count {
        env.storage().instance().set(&i, &i); // Should Warn
    }
}

// Note: while-loop bounds are not currently detected as parameter-derived
// by the lint's block-statement walker. This is a known limitation.
// Only for-loop bounds (via range expressions in desugared block stmts)
// are checked.

// --- Negative (should not warn): contract-controlled bound ---

fn good_constant_bound(env: Env) {
    for i in 0..10 {
        env.storage().instance().set(&i, &1); // Good — constant bound
    }
}

fn good_bounded_by_constant_array(env: Env) {
    let items = [1u32, 2, 3, 4, 5];
    for item in items.iter() {
        env.storage().instance().set(item, &1); // Good — bounded by array length
    }
}

// --- Negative (should not warn): validated input bound ---

// The lint only detects DIRECT parameter reads in the desugared range
// expression. Since `clamped` is a local variable (not a parameter),
// the lint does not fire even though `clamped` derives from `count`.
// This is the correct behavior: the contract validates the input before
// using it as a loop bound.
fn good_validated_bound(env: Env, count: u32) {
    let clamped = count.min(100); // Contract validates: max 100 iterations
    for i in 0..clamped {
        env.storage().instance().set(&i, &i); // Good — clamped before use
    }
}

// --- Suppression test ---

#[allow(unbounded_input_loop)]
fn allowed_unbounded_input(env: Env, count: u32) {
    for i in 0..count {
        env.storage().instance().set(&i, &i); // Good (allowed)
    }
}

fn main() {}
