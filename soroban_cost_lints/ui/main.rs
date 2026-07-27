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
        pub fn new() -> Vec { Vec }
        pub fn with_capacity(_n: u32) -> Vec { Vec }
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
// unnecessary_host_function_call — Fixtures
// =======================================================================

fn bad_host_call_in_loop(env: Env) {
    for _ in 0..10 {
        let _seq = env.ledger().sequence(); // Should Warn
    }
}

fn bad_host_call_in_while_loop(env: Env) {
    let mut i = 0;
    while i < 10 {
        let _seq = env.ledger().sequence(); // Should Warn
        i += 1;
    }
}

fn bad_host_call_in_loop_loop(env: Env) {
    loop {
        let _seq = env.ledger().sequence(); // Should Warn
        break;
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

fn bad_host_call_in_iterator_closure(env: Env) {
    let items = [1u32, 2, 3];
    items.iter().for_each(|_| {
        let _seq = env.ledger().sequence(); // Should Warn — called once per closure invocation
    });
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

// =======================================================================
// excessive_vec_capacity — Fixtures
// =======================================================================
// Positive (bad): calling Vec::with_capacity with a far larger capacity than
// the container will actually use wastes host memory and inflates the metered
// cost of the allocation.
// Negative (good): request no / little capacity up front and let growth
// happen naturally, or use Vec::new() for an empty container.

#[allow(excessive_vec_capacity)]
fn bad_excessive_vec_capacity() {
    let _v = Vec::with_capacity(1_000_000); // Should Warn — wildly excessive capacity
}

fn good_excessive_vec_capacity_fits_usage() {
    let _v = Vec::with_capacity(5); // Good — modest, sane capacity
}

#[allow(excessive_vec_capacity)]
fn allowed_excessive_vec_capacity() {
    let _v = Vec::with_capacity(1_000_000); // Good (allowed)
}

// =======================================================================
// expensive_crypto_in_loop — Fixtures
// =======================================================================
// Positive (bad): calling any Expensive host crypto operation inside a loop
// dispatches across the Wasm/host boundary per iteration. Even when the
// argument varies, the dispatch + setup overhead compounds with the actual
// hashing cost, so this is a structural smell.
// Negative (good): compute the hash once before / after the loop, or batch
// the inputs and hash a consolidated buffer.
//
// Note: the loop variable is intentionally passed into `sha256` so that the
// existing `unnecessary_host_function_call` lint (which only fires on
// loop-INDEPENDENT calls) does not also trigger and skew `main.stderr`.

#[allow(expensive_crypto_in_loop, unnecessary_host_function_call)]
fn bad_expensive_crypto_in_loop(env: Env) {
    let chunks: [[u8; 4]; 3] = [[1u8, 2, 3, 4], [5u8, 6, 7, 8], [9u8, 10, 11, 12]];
    for chunk in chunks.iter() {
        let _hash = env.crypto().sha256(chunk); // Should Warn
    }
}

fn good_expensive_crypto_called_once(env: Env) {
    let payload: [u8; 4] = [1, 2, 3, 4];
    let _hash = env.crypto().sha256(&payload); // Good — single call, no loop
}

#[allow(expensive_crypto_in_loop, unnecessary_host_function_call)]
fn allowed_expensive_crypto_in_loop(env: Env) {
    let chunks: [[u8; 4]; 3] = [[1u8, 2, 3, 4], [5u8, 6, 7, 8], [9u8, 10, 11, 12]];
    for chunk in chunks.iter() {
        let _hash = env.crypto().sha256(chunk); // Good (allowed)
    }
}

// =======================================================================
// redundant_storage_read — Fixtures
// =======================================================================
// Positive (bad): reading the same storage key more than once in the same
// function call burns additional ledger read accesses for no semantic gain.
// Negative (good): cache the value in a local binding when it is read more
// than once.
//
// Note: redundant reads are demonstrated OUTSIDE of a loop on purpose so
// that the existing `soroban_storage_in_loop` lint does not also fire.

#[allow(redundant_storage_read)]
fn bad_redundant_storage_read(env: Env) {
    let _a: Option<i32> = env.storage().instance().get(&1u32); // Should Warn — same key read twice
    let _b: Option<i32> = env.storage().instance().get(&1u32);
    let _c: Option<i32> = env.storage().instance().get(&1u32);
}

fn good_redundant_storage_read_cached(env: Env) {
    let _cached: Option<i32> = env.storage().instance().get(&1u32); // Good — single fetch
}

#[allow(redundant_storage_read)]
fn allowed_redundant_storage_read(env: Env) {
    let _a: Option<i32> = env.storage().instance().get(&1u32); // Good (allowed)
    let _b: Option<i32> = env.storage().instance().get(&1u32);
}

// =======================================================================
// unnecessary_vec_allocation — Fixtures
// =======================================================================
// Positive (bad): allocating a new Soroban SDK Vec when the value is never
// kept, never written to, or only used as a temporary, incurs a host-side
// allocation fee for no observable benefit.
// Negative (good): allocate only when the container is actually populated,
// reused, or returned. Prefer native `Vec` for in-memory scratch space.

#[allow(unnecessary_vec_allocation)]
fn bad_unnecessary_vec_allocation() {
    let _unused = Vec::new(); // Should Warn — created and immediately dropped, never written to
    let _another = Vec::new();
}

#[allow(unused_variables)]
fn good_necessary_vec_allocation_populated() {
    let mut v = Vec::new(); // Good — populated before being dropped
    v.push_back(1);
    v.push_back(2);
    let _populated = v;
}

#[allow(unnecessary_vec_allocation)]
fn allowed_unnecessary_vec_allocation() {
    let _unused = Vec::new(); // Good (allowed)
}

fn main() {}
