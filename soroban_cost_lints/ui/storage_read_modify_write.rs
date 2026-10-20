#![allow(
    storage_write_without_read,
    persistent_read_without_ttl_extension,
    instance_storage_for_unbounded_data,
    loop_invariant_storage_access,
    soroban_storage_in_loop,
    unbounded_input_loop,
    soroban_redundant_storage_read
)]

pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env {
        fn clone(&self) -> Self { Env }
    }
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

// ── FIRE: Two get/set cycles on the same key ──────────────────────────
fn two_cycles_same_key(env: Env) {
    let val: Option<i32> = env.storage().instance().get(&1);
    let new_val = val.unwrap_or(0) + 1;
    env.storage().instance().set(&1, &new_val); // first cycle (ok)

    let val2: Option<i32> = env.storage().instance().get(&1);
    let new_val2 = val2.unwrap_or(0) + 1;
    env.storage().instance().set(&1, &new_val2); // Should Warn — second cycle
}

// ── FIRE: Three cycles ────────────────────────────────────────────────
fn three_cycles_same_key(env: Env) {
    let val: Option<i32> = env.storage().instance().get(&1);
    env.storage().instance().set(&1, &(val.unwrap_or(0) + 1)); // first

    let val2: Option<i32> = env.storage().instance().get(&1);
    env.storage().instance().set(&1, &(val2.unwrap_or(0) + 1)); // Should Warn — second

    let val3: Option<i32> = env.storage().instance().get(&1);
    env.storage().instance().set(&1, &(val3.unwrap_or(0) + 1)); // Should Warn — third
}

// ── NO FIRE: Single cycle (correct pattern) ───────────────────────────
fn single_cycle(env: Env) {
    let val: Option<i32> = env.storage().instance().get(&1);
    let new_val = val.unwrap_or(0) + 1;
    env.storage().instance().set(&1, &new_val);
}

// ── NO FIRE: Cycles separated by a call that could touch storage ──────
fn helper_may_touch_storage(env: &Env) {
    env.storage().instance().set(&42, &99);
}

fn cycles_separated_by_call(env: Env) {
    let val: Option<i32> = env.storage().instance().get(&1);
    env.storage().instance().set(&1, &(val.unwrap_or(0) + 1));

    helper_may_touch_storage(&env); // may touch storage — resets tracking

    let val2: Option<i32> = env.storage().instance().get(&1);
    env.storage().instance().set(&1, &(val2.unwrap_or(0) + 1)); // Should NOT warn
}

// ── NO FIRE: Different keys ───────────────────────────────────────────
fn different_keys(env: Env) {
    let val_a: Option<i32> = env.storage().instance().get(&1);
    env.storage().instance().set(&1, &(val_a.unwrap_or(0) + 1));

    let val_b: Option<i32> = env.storage().instance().get(&2);
    env.storage().instance().set(&2, &(val_b.unwrap_or(0) + 1)); // different key — ok
}

// ── FIRE: Persistent storage cycles ───────────────────────────────────
fn persistent_two_cycles(env: Env) {
    let val: Option<i32> = env.storage().persistent().get(&1);
    env.storage().persistent().set(&1, &(val.unwrap_or(0) + 1));

    let val2: Option<i32> = env.storage().persistent().get(&1);
    env.storage().persistent().set(&1, &(val2.unwrap_or(0) + 1)); // Should Warn
}

fn main() {}
