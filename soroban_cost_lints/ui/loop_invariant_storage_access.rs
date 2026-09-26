#![allow(
    soroban_storage_in_loop,
    soroban_redundant_storage_read,
    storage_write_without_read,
    persistent_read_without_ttl_extension,
    instance_storage_for_unbounded_data,
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
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn extend_ttl<K>(&self, _k: &K, _ttl: &()) {}
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
use soroban_sdk::storage::Instance;

// Fires: a storage access inside a loop whose operands do not depend on the
// loop. `env` is a constant receiver, so every call in the chain
// (`storage` / `instance` / `get`) is loop-invariant.
fn invariant_get(env: Env) {
    for _ in 0..10 {
        let _: Option<i32> = env.storage().instance().get(&1); // Should Warn
    }
}

// Near-miss: the storage receiver `s` IS the loop variable, so the access
// depends on loop state and must NOT be flagged even though it is inside a loop.
fn varying_receiver(env: Env, stores: Vec<Instance>) {
    for s in stores.iter() {
        let _: Option<i32> = s.get(&1); // Should NOT warn
    }
}

// Partial: the *key* is a binding the loop body reassigns, so the `get` call
// itself is loop-varying and is NOT reported. This is the mutation path of the
// loop-dependence analysis, as opposed to the binding path covered by
// `varying_receiver`.
//
// The two calls that build the receiver (`env.storage()` and
// `env.storage().instance()`) do not read `key`, so they remain invariant and
// ARE reported. Loop-invariance is decided per call, not per statement.
fn varying_key_from_mutated_binding(env: Env, keys: Vec<u32>) {
    let mut key = 0u32;
    for k in keys.iter() {
        key = *k;
        let _: Option<i32> = env.storage().instance().get(&key); // Should NOT warn on the `get` call
    }
}

// Partial: same shape as above via a fresh per-iteration binding instead of a
// reassigned outer one. `let key = *k` introduces a loop-scoped binding, so
// `get(&key)` depends on loop state; the receiver chain is still invariant.
fn varying_key_from_iteration_local(env: Env, keys: Vec<u32>) {
    for k in keys.iter() {
        let key = *k;
        let _: Option<i32> = env.storage().instance().get(&key); // Should NOT warn on the `get` call
    }
}

// Fires: the accumulator is hoisted out of the loop, so nothing about the
// read varies per iteration and the whole chain is reported. This is the
// control for the two cases above — it proves they are quiet because the
// analysis is right, not because the statement is silent for some other
// reason.
fn invariant_key_from_outer_binding(env: Env) {
    let key = 7u32;
    for _ in 0..10 {
        let _: Option<i32> = env.storage().instance().get(&key); // Should Warn
    }
}

// Fires: a `while` loop is a loop, not only a `for` loop. The bound is a
// constant, so the body is invariant across iterations.
fn invariant_in_while_loop(env: Env) {
    let mut n = 0;
    while n < 10 {
        n += 1;
        let _: Option<i32> = env.storage().instance().get(&1); // Should Warn
    }
}

// Fires: `extend_ttl` is not one of the terminal read/write methods that the
// narrower `soroban_storage_in_loop` lint looks for, but it still crosses into
// the host, so this lint reports it too. (`extend_ttl_in_loop` also fires —
// that is a separate lint and is expected here.)
fn invariant_non_terminal_method(env: Env) {
    for _ in 0..10 {
        env.storage().persistent().extend_ttl(&1, &()); // Should Warn
    }
}

// Near-miss: a storage access inside a closure defined in a loop body. The
// closure is not a syntactic loop body, and the receiver is out of reach for
// the loop-dependence analysis, so nothing is reported — the conservative
// posture, since a closure may be invoked once or many times.
fn invariant_in_closure(env: Env) {
    for _ in 0..10 {
        let _f = || {
            let _: Option<i32> = env.storage().instance().get(&1); // Should NOT warn
        };
        let _ = _f;
    }
}

// Mixed: the outer loop's read is invariant, so the full chain is reported.
// The inner loop's read takes its key from the inner loop variable `i`, so
// only `get(&i)` is exempt — `env.storage()` and `env.storage().instance()`
// are still invariant with respect to `i` and are reported.
fn invariant_in_nested_loop(env: Env) {
    for _ in 0..10 {
        let _: Option<i32> = env.storage().instance().get(&1); // Should Warn
        for i in 0..3 {
            let _: Option<i32> = env.storage().instance().get(&i); // Should NOT warn on the `get` call
        }
    }
}

// Suppression: `#[allow]` must silence the lint, inherited by the
// standard attribute machinery rather than by anything this lint does.
#[allow(loop_invariant_storage_access)]
fn allowed_invariant_access(env: Env) {
    for _ in 0..10 {
        let _: Option<i32> = env.storage().instance().get(&1); // Should NOT warn (allowed)
    }
}

fn main() {}
