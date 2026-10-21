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
            // Return types are simplified to `()`; the lint matches these by
            // method name only (see `WRITE_METHODS` in the lint source).
            pub fn update<K, V>(&self, _k: &K, _f: impl FnOnce(Option<V>) -> V) {}
            pub fn try_update<K, V>(&self, _k: &K, _f: impl FnOnce(Option<V>) -> Option<V>) {}
            pub fn remove<K>(&self, _k: &K) {}
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn update<K, V>(&self, _k: &K, _f: impl FnOnce(Option<V>) -> V) {}
            pub fn try_update<K, V>(&self, _k: &K, _f: impl FnOnce(Option<V>) -> Option<V>) {}
            pub fn remove<K>(&self, _k: &K) {}
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn update<K, V>(&self, _k: &K, _f: impl FnOnce(Option<V>) -> V) {}
            pub fn try_update<K, V>(&self, _k: &K, _f: impl FnOnce(Option<V>) -> Option<V>) {}
            pub fn remove<K>(&self, _k: &K) {}
        }
    }

    /// Stand-in for `soroban_sdk::Set` so the collection-length bound
    /// fixture can exercise the `.len()` arm without depending on the real
    /// SDK: the lint only needs a type whose name matches a collection
    /// fragment and whose `len(&self) -> usize` method resolves.
    pub struct Set;
    impl Set {
        pub fn len(&self) -> usize { 0 }
    }

    // Models `soroban_sdk::Map`: the lint's `is_collection` test sees the
    // receiver of `keys()`/`values()`, so only the name and projections matter.
    pub struct Map<K, V>(core::marker::PhantomData<(K, V)>);
    impl<K: Clone, V: Clone> Map<K, V> {
        pub fn keys(&self) -> Vec<K> { Vec::new() }
        pub fn values(&self) -> Vec<V> { Vec::new() }
    }
}

use soroban_sdk::{Env, Map};

// =======================================================================
// unbounded_input_loop — Fixtures
// =======================================================================

// --- Positive (should warn): loop bound from caller input + storage write ---

fn bad_for_loop_bound_from_param(env: Env, count: u32) {
    for i in 0..count {
        env.storage().instance().set(&i, &i); // Should Warn
    }
}

/// A `while` loop bounded by a contract parameter: `count` is the caller's,
/// so the iteration count is too.
fn bad_while_loop_bound_from_param(env: Env, count: u32) {
    let mut i = 0;
    while i < count {
        env.storage().instance().set(&i, &i); // Should Warn
        i += 1;
    }
}

/// A bare `loop` whose exit is a `break` guarded by a parameter-derived
/// condition is just as caller-driven as a `while`.
fn bad_loop_break_on_param(env: Env, count: u32) {
    let mut i = 0;
    loop {
        env.storage().instance().set(&i, &i); // Should Warn
        i += 1;
        if i >= count {
            break;
        }
    }
}

// --- Negative (should not warn): contract-controlled bound ---

fn good_constant_bound(env: Env) {
    for i in 0..10 {
        env.storage().instance().set(&i, &1); // Good — constant bound
    }
}

/// A `while` loop with a constant bound is contract-controlled, not
/// caller-controlled.
fn good_while_constant_bound(env: Env) {
    let mut i = 0;
    while i < 10 {
        env.storage().instance().set(&i, &1); // Good — constant bound
        i += 1;
    }
}

/// A bare `loop` that exits on a constant comparison is not
/// caller-controlled either.
fn good_loop_constant_break(env: Env) {
    let mut i = 0;
    loop {
        env.storage().instance().set(&i, &1); // Good — constant exit condition
        i += 1;
        if i >= 10 {
            break;
        }
    }
}

/// A bare `loop` that breaks unconditionally has no caller-derived exit
/// condition at all, so its iteration count is fixed at one pass.
fn good_loop_unconditional_break(env: Env) {
    loop {
        env.storage().instance().set(&0, &1); // Good — no caller-derived exit
        break;
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

/// The same validation applies to a `while` loop: the bound here is the
/// local `clamped`, not the parameter.
fn good_while_validated_bound(env: Env, count: u32) {
    let clamped = count.min(100);
    let mut i = 0;
    while i < clamped {
        env.storage().instance().set(&i, &i); // Good — clamped before use
        i += 1;
    }
}

// --- Positive (should warn): SDK-current write and iterator spellings (#872) ---

/// `remove` deletes a ledger entry — a metered write. Iterating a
/// caller-supplied `Vec` parameter and removing one key per iteration is the
/// documented shape.
fn bad_persistent_remove_over_param_vec(env: Env, items: Vec<u32>) {
    for item in items {
        env.storage().persistent().remove(&item); // Should Warn
    }
}

/// `update` is a read-modify-write on a caller-named key.
fn bad_instance_update_over_map_keys(env: Env, roles: Map<u32, u32>) {
    for k in roles.keys() {
        env.storage().instance().update(&k, |prev| prev.unwrap_or(0) + 1); // Should Warn
    }
}

/// `try_update` is the fallible sibling of `update` — also a write.
fn bad_instance_try_update_over_map_values(env: Env, roles: Map<u32, u32>) {
    for v in roles.values() {
        // The explicit `Option<u32>` annotation is needed because the mock's
        // `try_update` leaves `V` inferrable only from the closure argument.
        env.storage().instance().try_update(&v, |prev: Option<u32>| prev); // Should Warn
    }
}

// --- Negative (should not warn): caller-driven iteration with only reads ---

/// `keys()` still marks caller-driven iteration, but `get`/`has` are not in
/// `WRITE_METHODS`, so a read-only loop stays silent. The `allow` silences the
/// sibling read lint, which fires orthogonally on `has()` without a TTL
/// extension; this fixture is about `unbounded_input_loop` only.
#[allow(persistent_read_without_ttl_extension)]
fn good_map_keys_loop_only_reads(env: Env, roles: Map<u32, u32>) {
    for k in roles.keys() {
        if env.storage().persistent().has(&k) {
            // Good — read-only body, no metered write per iteration.
            continue;
        }
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
