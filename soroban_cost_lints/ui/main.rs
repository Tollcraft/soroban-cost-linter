// Note: at this point four of the lints referenced in
// `#[allow(...)]` markers below (`excessive_vec_capacity`,
// `expensive_crypto_in_loop`, `redundant_storage_read`,
// `unnecessary_vec_allocation`) are not yet implemented — they exist as
// community-proposed follow-ups tracked in GitHub issues #59/#60/#61/#62.
// The unknown-lint allow forward-suppresses rustc warnings on those markers
// so we can land the fixtures today; once each lint lands, the corresponding
// `#[allow(<name>)]` becomes a real suppression with no edit needed.
#![allow(unknown_lints)]

pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env {
        fn clone(&self) -> Self { Env }
    }
    impl Env {
        pub fn storage(&self) -> storage::Storage {
            storage::Storage
        }
        pub fn ledger(&self) -> ledger::Ledger {
            ledger::Ledger
        }
        pub fn host(&self) -> host::Host {
            host::Host
        }
        pub fn crypto(&self) -> crypto::Crypto {
            crypto::Crypto
        }
        pub fn prng(&self) -> prng::Prng {
            prng::Prng
        }
        pub fn events(&self) -> events::Events {
            events::Events
        }
        pub fn deployer(&self) -> deploy::Deployer {
            deploy::Deployer
        }
        pub fn current_contract_address(&self) -> Address {
            Address
        }
        pub fn invoke_contract<T>(&self, _contract: &Address, _func: &Symbol, _args: ()) -> T
        where
            T: Default,
        {
            T::default()
        }
    }

    pub struct Address;
    impl Address {
        pub fn require_auth(&self) {}
        pub fn require_auth_for_args(&self, _args: &[Env]) {}
    }

    pub struct String;
    impl Clone for String {
        fn clone(&self) -> Self { String }
    }
    impl String {
        pub fn from_str(_env: &Env, _s: &str) -> String { String }
        pub fn to_bytes(&self) -> Bytes { Bytes(vec![]) }
    }

    pub mod storage{
        pub struct Storage;
        impl Storage {
            pub fn instance(&self) -> Instance { Instance }
            pub fn persistent(&self) -> Persistent { Persistent }
            pub fn temporary(&self) -> Temporary { Temporary }
        }

        pub struct Instance;
        impl Instance {
            pub fn get<K: ?Sized, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K: ?Sized, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K: ?Sized>(&self, _k: &K) -> bool { false }
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn extend_ttl<K>(&self, _k: &K, _threshold: &()) {}
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K: ?Sized, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K: ?Sized, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K: ?Sized>(&self, _k: &K) -> bool { false }
        }
    }

    pub mod ledger {
        pub struct Ledger;
        impl Ledger {
            pub fn sequence(&self) -> u32 { 0 }
        }
    }

    pub struct Bytes;
    impl Bytes {
        pub fn push_back(&mut self, _val: u8) {}
        pub fn append(&mut self, _other: &Bytes) {}
    }

    pub mod host {
        pub struct Host;
        impl Host {
            pub fn invoke_contract(&self) {}
            pub fn invoke_static(&self) {}
            pub fn budget_cloned(&self) {}
        }
    }

    // Tuple struct so `Bytes::from(_s)` and `Bytes(buf)` (HEAD's ineffective_bytes_concat) still work.
    // Also has `append` to support upstream's bytes_append_in_loop fixtures.
    pub struct Bytes(pub std::vec::Vec<u8>);
    impl Bytes {
        pub fn from(_s: &str) -> Bytes { Bytes(vec![]) }
        pub fn append(&mut self, _other: &Bytes) {}
    }
    impl std::ops::Add for Bytes {
        type Output = Bytes;
        fn add(self, _rhs: Bytes) -> Bytes { Bytes(vec![]) }
    }

    // Upstream's unit-struct Vec supports `push_back(i32)` for bytes_append_in_loop.
    // Extended with `get`, `len`, and `iter` so the vec_where_slice_could_be_used
    // fixtures can exercise read-only patterns.
    pub struct Vec;
    impl Vec {
        pub fn new() -> Vec { Vec }
        pub fn with_capacity(_n: u32) -> Vec { Vec }
        pub fn push_back(&mut self, _v: i32) {}
        pub fn get(&self, _i: u32) -> i32 { 0 }
        pub fn len(&self) -> u32 { 0 }
        // Returns a native std Vec to simulate iteration.
        pub fn iter(&self) -> std::vec::IntoIter<i32> { vec![1, 2, 3].into_iter() }
    }

    // HEAD's permissive Map: `insert<K, V>` is generic so map_insert_in_loop fixtures still work.
    pub struct Map;
    impl Map {
        pub fn insert<K, V>(&mut self, _k: K, _v: V) {}
        pub fn get<K: ?Sized, V>(&self, _k: &K) -> Option<V> { None }
    }

    pub struct Symbol;
    impl Symbol {
        pub fn new(_env: &Env, _s: &str) -> Symbol { Symbol }
    }

    pub mod vec {
        pub struct Vec<T>(std::marker::PhantomData<T>);
        impl<T> Vec<T> {
            pub fn contains(&self, _item: &T) -> bool { false }
            pub fn position(&self, _f: impl FnMut(&T) -> bool) -> Option<usize> { None }
            pub fn find(&self, _f: impl FnMut(&T) -> bool) -> Option<&T> { None }
        }
    }

    pub mod map {
        pub struct Map<K, V>(std::marker::PhantomData<(K, V)>);
        impl<K, V> Map<K, V> {
            pub fn contains_key(&self, _k: &K) -> bool { false }
            pub fn get(&self, _k: &K) -> Option<&V> { None }
        }
    }
}

use soroban_sdk::{Env, Symbol, vec::Vec, map::Map};


use soroban_sdk::Env;


// Realistic false-positive scenario: batch-writing different keys per iteration
#[allow(soroban_storage_in_loop)]
fn batch_write_different_keys(env: Env, pairs: &[(u32, u32)]) {
    for (key, val) in pairs {
        env.storage().instance().set(key, val); // Good (allowed) — different key each iteration
    }
}

// =======================================================================
// soroban_storage_in_loop — Inter-procedural Fixtures
// =======================================================================

fn persist(env: &Env) {
    env.storage().instance().set(&"key", &42);
}

fn noop(_env: &Env) {
    // nothing
}

fn bad_storage_through_call_in_loop(env: Env) {
    for _ in 0..10 {
        persist(&env); // Should Warn — callee performs storage
    }
}

fn good_noop_call_in_loop(env: Env) {
    for _ in 0..10 {
        noop(&env); // Good — callee does nothing costly
    }
}

fn good_storage_through_call_outside_loop(env: Env) {
    persist(&env); // Good — not inside a loop
}

#[allow(soroban_storage_in_loop)]
fn allowed_storage_through_call_in_loop(env: Env) {
    for _ in 0..10 {
        persist(&env); // Good (allowed)
    }
}

// =======================================================================
// loop_invariant_storage_access — Fixtures
// =======================================================================

// soroban_storage_in_loop suppressed so only loop_invariant_storage_access fires
#[allow(soroban_storage_in_loop)]
fn bad_invariant_storage_write_in_loop(env: Env) {
    for _i in 0..10 {
        env.storage().instance().set(&"constant_key", &42); // Should Warn (invariant write)
    }
}

#[allow(soroban_storage_in_loop)]
fn bad_invariant_storage_read_in_loop(env: Env) {
    for _i in 0..10 {
        let _val: Option<i32> = env.storage().persistent().get(&"constant_key"); // Should Warn (invariant read)
    }
}

#[allow(soroban_storage_in_loop)]
fn bad_invariant_storage_has_in_loop(env: Env) {
    loop {
        if env.storage().temporary().has(&"fixed") { // Should Warn (invariant check)
            break;
        }
    }
}

#[allow(soroban_storage_in_loop)]
fn good_variant_storage_write_in_loop(env: Env) {
    for i in 0..10 {
        env.storage().instance().set(&i, &i); // Good — key and value depend on loop variable
    }
}

#[allow(soroban_storage_in_loop)]
fn good_variant_storage_read_in_loop(env: Env) {
    for i in 0..10 {
        let _val: Option<i32> = env.storage().persistent().get(&i); // Good — key depends on loop variable
    }
}

#[allow(soroban_storage_in_loop)]
fn good_variant_storage_has_in_loop(env: Env) {
    let mut n = 0;
    while n < 10 {
        if env.storage().temporary().has(&n) { // Good — key depends on mutated variable
            n += 1;
        } else {
            break;
        }
    }
}

fn good_invariant_storage_outside_loop(env: Env) {
    env.storage().instance().set(&"key", &42); // Good — not inside a loop
}

#[allow(loop_invariant_storage_access)]
fn allowed_invariant_storage_in_loop(env: Env) {
    for _i in 0..10 {
        env.storage().instance().set(&"key", &42); // Good (allowed)
    }
}

// =======================================================================
// redundant_env_clone — Fixtures
// =======================================================================

fn bad_clone_env(env: Env) {
    let _cloned = env.clone(); // Should Warn
}

fn good_no_clone_needed(env: Env) {
    let _ref = &env; // Good — no clone, just a reference
}

fn bad_clone_env_ufcs_env(env: Env) {
    let _cloned = Env::clone(&env); // Should Warn
}

fn bad_clone_env_ufcs_clone(env: Env) {
    let _cloned = Clone::clone(&env); // Should Warn
}

fn good_env_ref_clone(env: &Env) {
    let _cloned = env.clone(); // Good — &Env, clone produces owned Env
}

fn good_env_used_after_clone(env: Env) {
    let _cloned = env.clone(); // Good — env used after on next line
    let _also_env = env;
}

fn good_fn_takes_env_by_value(env: Env) {
    let cloned = env.clone(); // Good — env used after the clone
    takes_env(cloned);
    let _still_here = env;
}

fn takes_env(_e: Env) {}

#[allow(redundant_env_clone)]
fn allowed_clone_env(env: Env) {
    let _cloned = env.clone(); // Good (allowed)
}



// =======================================================================
// symbol_new_for_short_literal — Fixtures
// =======================================================================

fn bad_symbol_new_short_literal(env: Env) {
    let _sym = Symbol::new(&env, "hello"); // Should Warn - 5 chars, valid
}

fn bad_symbol_new_9_chars(env: Env) {
    let _sym = Symbol::new(&env, "abcdefghi"); // Should Warn - exactly 9 chars
}

fn bad_symbol_new_with_underscore(env: Env) {
    let _sym = Symbol::new(&env, "hello_world"); // Should Warn - 11 chars but only 9 allowed
}

fn bad_symbol_new_short_with_underscore(env: Env) {
    let _sym = Symbol::new(&env, "hello_wor"); // Should Warn - 9 chars with underscore
}

fn good_symbol_new_too_long(env: Env) {
    let _sym = Symbol::new(&env, "hello_world"); // Good - 11 chars > 9
}

fn good_symbol_new_invalid_chars(env: Env) {
    let _sym = Symbol::new(&env, "hello-world"); // Good - contains invalid char '-'
}

fn good_symbol_new_non_literal(env: Env) {
    let s = "hello";
    let _sym = Symbol::new(&env, s); // Good - not a literal
}

fn good_symbol_new_empty(env: Env) {
    let _sym = Symbol::new(&env, ""); // Good - empty string
}

#[allow(symbol_new_for_short_literal)]
fn allowed_symbol_new_short_literal(env: Env) {
    let _sym = Symbol::new(&env, "hello"); // Good (allowed)
}

// =======================================================================
// contract_call_in_loop — Fixtures
// =======================================================================

fn bad_invoke_contract_in_for_loop(env: Env, addr: soroban_sdk::Address, func: Symbol) {
    for _ in 0..10 {
        let _: i32 = env.invoke_contract(&addr, &func, ()); // Should Warn
    }
}

fn bad_invoke_contract_in_while_loop(env: Env, addr: soroban_sdk::Address, func: Symbol) {
    let mut i = 0;
    while i < 10 {
        let _: i32 = env.invoke_contract(&addr, &func, ()); // Should Warn
        i += 1;
    }
}

fn bad_invoke_contract_in_loop_loop(env: Env, addr: soroban_sdk::Address, func: Symbol) {
    loop {
        let _: i32 = env.invoke_contract(&addr, &func, ()); // Should Warn
        break;
    }
}

fn good_invoke_contract_outside_loop(env: Env, addr: soroban_sdk::Address, func: Symbol) {
    let _: i32 = env.invoke_contract(&addr, &func, ()); // Good — single call, not in a loop
}

#[allow(contract_call_in_loop)]
fn allowed_invoke_contract_in_loop(env: Env, addr: soroban_sdk::Address, func: Symbol) {
    for _ in 0..10 {
        let _: i32 = env.invoke_contract(&addr, &func, ()); // Good (allowed)
    }
}

// =======================================================================
// unnecessary_string_to_bytes — Fixtures
// =======================================================================

fn bad_persistent_read_no_ttl_extension(env: Env) {
    let _val: Option<i32> = env.storage().persistent().get(&1); // Should Warn
}

fn bad_persistent_has_no_ttl_extension(env: Env) {
    if env.storage().persistent().has(&1) { // Should Warn
    }
}

fn good_persistent_read_with_ttl_extension(env: Env) {
    env.storage().persistent().extend_ttl(&1, &());
    let _val: Option<i32> = env.storage().persistent().get(&1); // Good
}

fn good_instance_read(env: Env) {
    let _val: Option<i32> = env.storage().instance().get(&1); // Good — not persistent
}

fn good_temporary_read(env: Env) {
    let _val: Option<i32> = env.storage().temporary().get(&1); // Good — not persistent
}

#[allow(persistent_read_without_ttl_extension)]
fn allowed_persistent_read(env: Env) {
    let _val: Option<i32> = env.storage().persistent().get(&1); // Good (allowed)
}

fn main() {}

// =======================================================================
// soroban_redundant_storage_read — Fixtures
// =======================================================================

fn bad_sequential_get_same_key(env: Env, key: i32) {
    let _a: Option<i32> = env.storage().instance().get(&key);
    let _b: Option<i32> = env.storage().instance().get(&key); // Should Warn
}

fn bad_sequential_has_then_get(env: Env, key: i32) {
    let exists = env.storage().instance().has(&key);
    let _val: Option<i32> = env.storage().instance().get(&key); // Should Warn
}

fn bad_sequential_has_then_has(env: Env, key: i32) {
    let _a = env.storage().instance().has(&key);
    let _b = env.storage().instance().has(&key); // Should Warn
}

fn bad_sequential_persistent_get(env: Env, key: i32) {
    let _a: Option<i32> = env.storage().persistent().get(&key);
    let _b: Option<i32> = env.storage().persistent().get(&key); // Should Warn
}

fn bad_sequential_temporary_get(env: Env, key: i32) {
    let _a: Option<i32> = env.storage().temporary().get(&key);
    let _b: Option<i32> = env.storage().temporary().get(&key); // Should Warn
}

fn good_set_resets_tracking(env: Env, key: i32) {
    let _a: Option<i32> = env.storage().instance().get(&key);
    env.storage().instance().set(&key, &1);
    let _b: Option<i32> = env.storage().instance().get(&key); // Good — write in between
}

fn good_different_keys(env: Env, key1: i32, key2: i32) {
    let _a: Option<i32> = env.storage().instance().get(&key1);
    let _b: Option<i32> = env.storage().instance().get(&key2); // Good — different key
}

fn good_single_read(env: Env, key: i32) {
    let _a: Option<i32> = env.storage().instance().get(&key); // Good — only one read
}

#[allow(soroban_redundant_storage_read)]
fn allowed_sequential_read(env: Env, key: i32) {
    let _a: Option<i32> = env.storage().instance().get(&key);
    let _b: Option<i32> = env.storage().instance().get(&key); // Good (allowed)
}
