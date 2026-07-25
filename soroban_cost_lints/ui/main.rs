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
}

use soroban_sdk::Env;

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

fn main() {}

// =======================================================================
// soroban_inefficient_bytes_concat — Fixtures
// =======================================================================

fn bad_bytes_push_back_in_loop() {
    let mut bytes = soroban_sdk::Bytes;
    for i in 0..10 {
        bytes.push_back(i as u8); // Should Warn
    }
}

fn bad_bytes_append_in_loop() {
    let mut bytes = soroban_sdk::Bytes;
    let other = soroban_sdk::Bytes;
    for _ in 0..10 {
        bytes.append(&other); // Should Warn
    }
}

fn bad_bytes_push_back_in_while() {
    let mut bytes = soroban_sdk::Bytes;
    let mut i = 0;
    while i < 10 {
        bytes.push_back(i as u8); // Should Warn
        i += 1;
    }
}

fn bad_bytes_append_in_loop_loop() {
    let mut bytes = soroban_sdk::Bytes;
    let other = soroban_sdk::Bytes;
    loop {
        bytes.append(&other); // Should Warn
        break;
    }
}

fn good_bytes_concat_outside_loop() {
    let mut bytes = soroban_sdk::Bytes;
    bytes.push_back(1); // Good — outside loop
    for _ in 0..10 {
        let _x = 1;
    }
}

fn good_vec_build_then_convert() {
    let mut v: Vec<u8> = Vec::new();
    for i in 0..10 {
        v.push(i as u8); // Good — Vec<u8> is not Bytes
    }
}

#[allow(soroban_inefficient_bytes_concat)]
fn allowed_bytes_concat_in_loop() {
    let mut bytes = soroban_sdk::Bytes;
    for i in 0..10 {
        bytes.push_back(i as u8); // Good (allowed)
    }
}
