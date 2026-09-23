#![allow(
    soroban_storage_in_loop,
    storage_write_without_read,
    redundant_env_clone,
    symbol_new_for_short_literal,
    storage_key_construction_in_loop
)]

pub mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn ledger(&self) -> ledger::Ledger {
            ledger::Ledger
        }
    }

    pub mod ledger {
        pub struct Ledger;
        impl Ledger {
            pub fn sequence(&self) -> u32 { 0 }
        }
    }

    pub struct Symbol;
    impl Symbol {
        pub fn new(_env: &Env, _s: &str) -> Symbol { Symbol }
    }
}

use soroban_sdk::{Env, Symbol};

// =======================================================================
// symbol_key_event_topics — Fixtures
// =======================================================================

// --- Positive (should warn): re-constructed symbols for event topics ---

fn bad_event_topic_symbol_new(env: Env) {
    let _topic = Symbol::new(&env, "transfer");
    let _topic2 = Symbol::new(&env, "transfer");
}

fn bad_event_topic_long_repeat(env: Env) {
    let _topic = Symbol::new(&env, "payable");
    let _topic2 = Symbol::new(&env, "payable");
}

// --- Negative (should not warn): single use of a symbol ---

fn good_event_topic_single(env: Env) {
    let _topic = Symbol::new(&env, "transfer");
}

fn main() {}