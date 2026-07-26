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
    }

    pub struct Address;

    pub mod storage {
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
            pub fn get<K: ?Sized, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K: ?Sized, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K: ?Sized>(&self, _k: &K) -> bool { false }
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

    pub mod crypto {
        pub struct Crypto;
        impl Crypto {
            pub fn sha256(&self, _data: &[u8]) -> [u8; 32] { [0; 32] }
            pub fn keccak256(&self, _data: &[u8]) -> [u8; 32] { [0; 32] }
        }
    }

    pub mod prng {
        pub struct Prng;
        impl Prng {
            pub fn u64_in_range(&self, _lo: u64, _hi: u64) -> u64 { 0 }
        }
    }

    pub mod events {
        pub struct Events;
        impl Events {
            pub fn publish<T, D>(&self, _topics: T, _data: D) {}
        }
    }

    pub mod deploy {
        pub struct Deployer;
        impl Deployer {
            pub fn uploaded_wasm_hash(&self) -> [u8; 32] { [0; 32] }
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
    pub struct Vec;
    impl Vec {
        pub fn push_back(&mut self, _v: i32) {}
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
}

use soroban_sdk::{Bytes, Env, Map, Symbol, Vec};

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

// Realistic false-positive scenario: batch-writing different keys per iteration
#[allow(soroban_storage_in_loop)]
fn batch_write_different_keys(env: Env, pairs: &[(u32, u32)]) {
    for (key, val) in pairs {
        env.storage().instance().set(key, val); // Good (allowed) — different key each iteration
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

fn bad_crypto_call_in_loop(env: Env) {
    let data = [1u8, 2, 3];
    for _ in 0..10 {
        let _hash = env.crypto().sha256(&data); // Should Warn — same input every iteration
    }
}

fn good_crypto_call_on_loop_variable(env: Env) {
    let data = [[1u8; 4], [2u8; 4], [3u8; 4]];
    for chunk in data.iter() {
        let _hash = env.crypto().sha256(chunk); // Good — hashes a different input each iteration
    }
}

fn good_crypto_call_indexed_by_counter(env: Env) {
    let data = [1u8, 2, 3];
    let mut i = 0;
    while i < 3 {
        let _hash = env.crypto().keccak256(&data[i..]); // Good — argument moves with the counter
        i += 1;
    }
}

fn bad_prng_call_in_loop(env: Env) {
    for _ in 0..10 {
        let _n = env.prng().u64_in_range(0, 100); // Should Warn
    }
}

fn bad_current_contract_address_in_loop(env: Env) {
    for _ in 0..10 {
        let _addr = env.current_contract_address(); // Should Warn
    }
}

fn good_events_publish_of_loop_value(env: Env) {
    for i in 0..10 {
        env.events().publish((i,), i); // Good — publishes the value of this iteration
    }
}

fn good_deployer_call_outside_loop(env: Env) {
    let hash = env.deployer().uploaded_wasm_hash(); // Good — called once before the loop
    for _ in 0..10 {
        let _hash = hash;
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
// storage_write_without_read — Fixtures
// =======================================================================

fn bad_storage_write_without_read(env: Env) {
    env.storage().instance().set(&"key1", &1); // Should Warn — no prior read
}

fn good_storage_write_with_read(env: Env) {
    let _: Option<i32> = env.storage().instance().get(&"key1"); // Read first
    env.storage().instance().set(&"key1", &1); // Good — read before write
}

fn good_storage_write_with_has(env: Env) {
    let _exists = env.storage().instance().has(&"key1"); // Check first
    env.storage().instance().set(&"key1", &1); // Good — has before write
}

#[allow(storage_write_without_read)]
fn allowed_storage_write_without_read(env: Env) {
    env.storage().instance().set(&"key1", &1); // Good (allowed)
}

// =======================================================================
// inefficient_bytes_concat — Fixtures
// =======================================================================

fn bad_inefficient_bytes_concat(env: Env) {
    let mut result = Bytes::from("");
    for _ in 0..10 {
        result = result + Bytes::from("x"); // Should Warn
    }
}

fn good_efficient_bytes_concat(env: Env) {
    let mut buf: std::vec::Vec<u8> = std::vec::Vec::new();
    for _ in 0..10 {
        buf.extend_from_slice(b"x"); // Good — aggregate in Vec first
    }
    let _result = Bytes(buf);
}

#[allow(inefficient_bytes_concat)]
fn allowed_inefficient_bytes_concat(env: Env) {
    let mut result = Bytes::from("");
    for _ in 0..10 {
        result = result + Bytes::from("x"); // Good (allowed)
    }
}

// =======================================================================
// map_insert_in_loop — Fixtures
// =======================================================================

fn bad_map_insert_in_loop(env: Env) {
    let mut map = Map;
    for i in 0..10 {
        map.insert(&i, &1); // Should Warn
    }
}

fn good_map_insert_outside_loop(env: Env) {
    let mut map = Map;
    map.insert(&1, &1); // Good — outside the loop
    for i in 0..10 {
        let _: Option<i32> = map.get(&i);
    }
}

#[allow(map_insert_in_loop)]
fn allowed_map_insert_in_loop(env: Env) {
    let mut map = Map;
    for i in 0..10 {
        map.insert(&i, &1); // Good (allowed)
    }
}

// =======================================================================
// bytes_append_in_loop — Fixtures
// =======================================================================

fn bad_bytes_append_in_for_loop() {
    let mut bytes = Bytes(vec![]);
    for _ in 0..10 {
        bytes.append(&Bytes(vec![])); // Should Warn
    }
}

fn bad_vec_push_back_in_while_loop() {
    let mut v = Vec;
    let mut i = 0;
    while i < 10 {
        v.push_back(i); // Should Warn
        i += 1;
    }
}

fn good_single_append_outside_loop() {
    let mut bytes = Bytes(vec![]);
    bytes.append(&Bytes(vec![])); // Good - single append outside loop
}

fn main() {}
