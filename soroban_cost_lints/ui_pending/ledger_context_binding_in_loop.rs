pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env {
        fn clone(&self) -> Self { Env }
    }
    impl Env {
        pub fn ledger(&self) -> ledger::Ledger {
            ledger::Ledger
        }
    }

    pub mod ledger {
        pub struct Ledger;
        impl Ledger {
            pub fn sequence(&self) -> u32 { 0 }
            pub fn timestamp(&self) -> u64 { 0 }
            pub fn network_id(&self) -> [u8; 32] { [0u8; 32] }
        }
    }
}

use soroban_sdk::Env;

// =======================================================================
// ledger_context_read_in_loop via local binding — Fixtures
// =======================================================================

// --- Triggering cases: ledger reads inside loops via local `let ledger =` binding ---

fn bad_sequence_via_binding_for(env: Env) {
    let ledger = env.ledger();
    for _ in 0..10 {
        let _seq = ledger.sequence(); // Should Warn
    }
}

fn bad_timestamp_via_binding_while(env: Env) {
    let ledger = env.ledger();
    let mut i = 0;
    while i < 10 {
        let _ts = ledger.timestamp(); // Should Warn
        i += 1;
    }
}

fn bad_network_id_via_binding_loop(env: Env) {
    let ledger = env.ledger();
    loop {
        let _nid = ledger.network_id(); // Should Warn
        break;
    }
}

fn bad_all_accessors_via_binding(env: Env) {
    let ledger = env.ledger();
    for _ in 0..3 {
        let _seq = ledger.sequence();  // Should Warn
        let _ts = ledger.timestamp();  // Should Warn
    }
}

// --- Non-triggering cases: binding read outside loop ---

fn good_binding_read_outside_loop(env: Env) {
    let ledger = env.ledger();
    let _seq = ledger.sequence(); // Good — outside any loop
    let _ts = ledger.timestamp(); // Good — outside any loop
}

fn good_binding_hoisted(env: Env) {
    let ledger = env.ledger();
    let seq = ledger.sequence(); // Good — hoisted result
    for _ in 0..10 {
        let _ = seq;
    }
}

#[allow(ledger_context_read_in_loop)]
fn allowed_binding_in_loop(env: Env) {
    let ledger = env.ledger();
    for _ in 0..10 {
        let _seq = ledger.sequence(); // Good (allowed)
    }
}

fn main() {}
