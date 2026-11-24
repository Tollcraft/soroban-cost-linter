#![allow(
    storage_write_without_read,
    persistent_read_without_ttl_extension,
    instance_storage_for_unbounded_data,
    loop_invariant_storage_access,
    soroban_storage_in_loop,
    unbounded_input_loop
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
            pub fn try_get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn remove<K>(&self, _k: &K) {}
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn try_get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn remove<K>(&self, _k: &K) {}
            pub fn extend_ttl<K>(&self, _k: &K, _ttl: &()) {}
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn try_get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn remove<K>(&self, _k: &K) {}
        }
    }
}

use soroban_sdk::Env;

// Fires: two reads of the same key with no intervening write. The second read is
// flagged as redundant.
fn redundant_read(env: Env) {
    let _: Option<i32> = env.storage().instance().get(&1);
    let _: Option<i32> = env.storage().instance().get(&1); // Should Warn
}

// Fires: `has` observes the key just like `get`, so the following `get` repeats
// work that has already been metered.
fn redundant_read_after_has(env: Env) {
    let _ = env.storage().instance().has(&1);
    let _: Option<i32> = env.storage().instance().get(&1); // Should Warn
}

// Fires: the same redundant pair on persistent storage rather than instance.
fn redundant_read_persistent(env: Env) {
    let _: Option<i32> = env.storage().persistent().get(&1);
    let _: Option<i32> = env.storage().persistent().get(&1); // Should Warn
}

// Fires: try_get is also a metered read and repeats the same key without an
// intervening write.
fn redundant_try_get(env: Env) {
    let _: Option<i32> = env.storage().instance().try_get(&1);
    let _: Option<i32> = env.storage().instance().try_get(&1); // Should Warn
}

// --- Interleaved keys: each key is tracked independently ---

fn interleaved_keys(env: Env) {
    let a1: Option<i32> = env.storage().instance().get(&1);
    let b1: Option<i32> = env.storage().instance().get(&2);
    let a2: Option<i32> = env.storage().instance().get(&1); // Should Warn
    let b2: Option<i32> = env.storage().instance().get(&2); // Should Warn
    let _ = (a1, b1, a2, b2);
}

// --- A `remove` invalidates a tracked read just like a `set` ---

// Near-miss: the removal changed the observable value, so re-reading the key is
// new work and must NOT be flagged.
fn remove_clears_tracked_key(env: Env) {
    let first: Option<i32> = env.storage().instance().get(&1);
    env.storage().instance().remove(&1);
    let second: Option<i32> = env.storage().instance().get(&1); // Should NOT warn
    let _ = (first, second);
}

// Fires: after the removal the first re-read is new state, so the second one is
// redundant again -- exactly one warning, on the final read.
fn remove_then_repeated_read(env: Env) {
    let first: Option<i32> = env.storage().persistent().get(&1);
    env.storage().persistent().remove(&1);
    let second: Option<i32> = env.storage().persistent().get(&1); // Should NOT warn
    let third: Option<i32> = env.storage().persistent().get(&1); // Should Warn
    let _ = (first, second, third);
}

// Near-miss: three different keys interleave without a single repeat.
fn three_distinct_keys(env: Env) {
    let a: Option<i32> = env.storage().instance().get(&1);
    let b: Option<i32> = env.storage().instance().get(&2);
    let c: Option<i32> = env.storage().instance().get(&3);
    let _ = (a, b, c);
}

// Near-miss: a `set` clears every tracked key, so the second read of key 2 is
// not a repeat even though the write targeted key 1.
fn write_clears_all_tracked_keys(env: Env) {
    let a: Option<i32> = env.storage().instance().get(&1);
    let b: Option<i32> = env.storage().instance().get(&2);
    env.storage().instance().set(&1, &3);
    let b2: Option<i32> = env.storage().instance().get(&2); // Should NOT warn
    let _ = (a, b, b2);
}

// Fires: a `get` followed by `try_get` is still the same read, just observed by a
// different accessor method.
fn redundant_get_then_try_get(env: Env) {
    let _: Option<i32> = env.storage().instance().get(&1);
    let _: Option<i32> = env.storage().instance().try_get(&1); // Should Warn
}

// Near-miss: a write between the reads clears the tracked key, so the second
// read is not redundant and must NOT be flagged.
fn read_write_read(env: Env) {
    let _: Option<i32> = env.storage().instance().get(&1);
    env.storage().instance().set(&1, &2);
    let _: Option<i32> = env.storage().instance().get(&1); // Should NOT warn
}

// Near-miss: the two reads use different keys, so the second is not redundant.
fn different_keys(env: Env) {
    let _: Option<i32> = env.storage().instance().get(&1); // Should NOT warn
    let _: Option<i32> = env.storage().instance().get(&2); // Should NOT warn
}

// Near-miss: the same key read through two different storage accessors is not
// the same read, because the accessor is part of the tracked identity.
fn different_accessor_same_key(env: Env) {
    let _: Option<i32> = env.storage().instance().get(&1); // Should NOT warn
    let _: Option<i32> = env.storage().persistent().get(&1); // Should NOT warn
}

// Near-miss: the read inside the `if` arm lives in its own block with a fresh
// tracker, so the repeat of the same key in the enclosing block is never
// compared to it and must NOT be flagged.
fn nested_if_arm(env: Env, cond: bool) {
    if cond {
        let inner: Option<i32> = env.storage().instance().get(&1);
        let _ = inner;
    }
    let outer: Option<i32> = env.storage().instance().get(&1); // Should NOT warn
    let _ = outer;
}

// Near-miss: the read inside the loop body lives in its own block with a fresh
// tracker, so the repeat of the same key after the loop is never compared to
// it and must NOT be flagged.
fn nested_loop_body(env: Env) {
    for _ in 0..2 {
        let inner: Option<i32> = env.storage().instance().get(&1);
        let _ = inner;
    }
    let outer: Option<i32> = env.storage().instance().get(&1); // Should NOT warn
    let _ = outer;
}

fn main() {}
