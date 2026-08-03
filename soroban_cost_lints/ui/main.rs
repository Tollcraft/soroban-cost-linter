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

    // Returned by `Env`'s accessors and matched by SOROBAN_HOST_TYPES in the
    // lint source. Deleted by a merge while the accessors kept referencing them.
    pub mod crypto {
        pub struct Crypto;
        impl Crypto {
            pub fn sha256(&self, _data: &[u8]) -> [u8; 32] { [0; 32] }
            pub fn keccak256(&self, _data: &[u8]) -> [u8; 32] { [0; 32] }
            pub fn ed25519_verify(&self, _key: &[u8], _msg: &[u8], _sig: &[u8]) {}
        }
    }

    pub mod prng {
        pub struct Prng;
        impl Prng {
            pub fn u64_in_range(&self, _low: u64, _high: u64) -> u64 { 0 }
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
            pub fn with_current_contract(&self, _salt: [u8; 32]) -> Deployer { Deployer }
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
    // One tuple struct carrying every method the fixtures need. A merge left
    // two separate `Bytes` definitions here, which stopped this file compiling.
    pub struct Bytes(pub std::vec::Vec<u8>);
    impl Bytes {
        pub fn from(_s: &str) -> Bytes { Bytes(vec![]) }
        pub fn append(&mut self, _other: &Bytes) {}
        pub fn push_back(&mut self, _val: u8) {}
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

use soroban_sdk::{Bytes, Env, Map, Symbol, Vec};




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

fn good_single_append_outside_loop() {
    let mut bytes = Bytes(vec![]);
    bytes.append(&Bytes(vec![])); // Good - single append outside loop
}

// =======================================================================
// signature_verification_in_loop — Fixtures
// =======================================================================

fn bad_signature_verification_in_for_loop(env: Env) {
    let key = [0u8; 32];
    let msg = [0u8; 32];
    let sig = [0u8; 64];
    for _ in 0..10 {
        env.crypto().ed25519_verify(&key, &msg, &sig); // Should Warn
    }
}

fn good_signature_verification_outside_loop(env: Env) {
    let key = [0u8; 32];
    let msg = [0u8; 32];
    let sig = [0u8; 64];
    env.crypto().ed25519_verify(&key, &msg, &sig); // Good — called once
}

#[allow(signature_verification_in_loop)]
fn allowed_signature_verification_in_loop(env: Env) {
    let key = [0u8; 32];
    let msg = [0u8; 32];
    let sig = [0u8; 64];
    for _ in 0..10 {
        env.crypto().ed25519_verify(&key, &msg, &sig); // Good (allowed)
    }
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

fn main() {}
