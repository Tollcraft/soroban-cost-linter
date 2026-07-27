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
            pub fn try_get<K, V>(&self, _k: &K) -> Option<Option<V>> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn remove<K>(&self, _k: &K) {}
            pub fn update<K, V>(&self, _k: &K, _f: fn(Option<V>) -> Option<V>) {}
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn try_get<K, V>(&self, _k: &K) -> Option<Option<V>> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn remove<K>(&self, _k: &K) {}
            pub fn update<K, V>(&self, _k: &K, _f: fn(Option<V>) -> Option<V>) {}
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn try_get<K, V>(&self, _k: &K) -> Option<Option<V>> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn remove<K>(&self, _k: &K) {}
            pub fn update<K, V>(&self, _k: &K, _f: fn(Option<V>) -> Option<V>) {}
        }
    }

    pub mod ledger {
        pub struct Ledger;
        impl Ledger {
            pub fn sequence(&self) -> u32 { 0 }
        }
    }

    pub mod host {
        pub struct Host;
        impl Host {
            pub fn invoke_contract(&self) {}
            pub fn invoke_static(&self) {}
            pub fn budget_cloned(&self) {}
        }
    }

    pub struct Symbol;
    impl Symbol {
        pub fn new(_env: &Env, _s: &str) -> Symbol { Symbol }
    }
}

use soroban_sdk::{Env, Symbol};

// =======================================================================
// soroban_storage_in_loop — Fixtures
// =======================================================================

fn bad_storage_in_for_loop(env: Env) {
    for i in 0..10 {
        env.storage().instance().set(&i, &1); // Should Warn
    }
}

fn bad_storage_in_while_loop(env: Env) {
    let mut i = 0;
    while i < 10 {
        let _: Option<i32> = env.storage().persistent().get(&i); // Should Warn
        i += 1;
    }
}

fn bad_storage_in_loop_loop(env: Env) {
    loop {
        if env.storage().temporary().has(&1) { // Should Warn
            break;
        }
    }
}

fn good_storage_outside_loop(env: Env) {
    env.storage().instance().set(&1, &1); // Good
}

#[allow(soroban_storage_in_loop)]
fn allowed_storage_in_loop(env: Env) {
    for i in 0..10 {
        env.storage().instance().set(&i, &1); // Good (allowed)
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

#[allow(redundant_env_clone)]
fn allowed_clone_env(env: Env) {
    let _cloned = env.clone(); // Good (allowed)
}

// =======================================================================
// unnecessary_host_function_call — Fixtures
// =======================================================================

fn bad_host_call_in_loop(env: Env) {
    for _ in 0..10 {
        let _seq = env.ledger().sequence(); // Should Warn
    }
}

fn good_host_call_outside_loop(env: Env) {
    let seq = env.ledger().sequence(); // Good — called once before the loop
    for _ in 0..10 {
        let _seq = seq;
    }
}

#[allow(unnecessary_host_function_call)]
fn allowed_host_call_in_loop(env: Env) {
    for _ in 0..10 {
        let _seq = env.ledger().sequence(); // Good (allowed)
    }
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
// blind_storage_write — Fixtures
// =======================================================================

fn bad_blind_write_instance(env: Env) {
    env.storage().instance().set(&"alpha", &1u32); // Should Warn
}

fn bad_blind_write_persistent(env: Env) {
    let k = "beta";
    env.storage().persistent().set(&k, &2u32); // Should Warn
}

fn bad_blind_write_temporary(env: Env) {
    let k = "gamma";
    env.storage().temporary().set(&k, &3u32); // Should Warn
}

fn good_write_after_get(env: Env) {
    let _existing: Option<u32> = env.storage().instance().get(&"alpha"); // read first
    env.storage().instance().set(&"alpha", &1u32); // Good - same key was read
}

fn good_write_after_has(env: Env) {
    if env.storage().persistent().has(&"beta") {
        env.storage().persistent().set(&"beta", &2u32); // Good - has() before set
    }
}

fn good_write_after_try_get(env: Env) {
    let _existing: Option<Option<u32>> = env.storage().temporary().try_get(&"gamma");
    env.storage().temporary().set(&"gamma", &3u32); // Good - try_get before set
}

fn good_write_after_remove(env: Env) {
    let _removed: Option<u32> = env.storage().instance().get(&"alpha");
    env.storage().instance().remove(&"alpha");
    env.storage().instance().set(&"alpha", &1u32); // Good - read before set
}

fn good_write_after_update(env: Env) {
    fn bump(_existing: Option<u32>) -> Option<u32> { Some(2) }
    env.storage().persistent().update(&"beta", bump); // read inside update
    env.storage().persistent().set(&"beta", &2u32); // Good - update before set
}

fn bad_blind_write_different_buckets(env: Env) {
    let _ = env.storage().instance().get(&"shared"); // read on instance only
    env.storage().persistent().set(&"shared", &1u32); // Should Warn - read was on a different bucket
}

#[allow(blind_storage_write)]
fn allowed_blind_write(env: Env) {
    env.storage().instance().set(&"alpha", &1u32); // Good (allowed)
}

fn main() {}