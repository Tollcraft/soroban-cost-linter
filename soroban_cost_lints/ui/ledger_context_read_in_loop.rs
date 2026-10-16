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
// ledger_context_read_in_loop — Fixtures
// =======================================================================

// --- Triggering cases (ledger reads inside loops) ---

fn bad_sequence_in_for_loop(env: Env) {
    for _ in 0..10 {
        let _seq = env.ledger().sequence(); // Should Warn
    }
}

fn bad_timestamp_in_while_loop(env: Env) {
    let mut i = 0;
    while i < 10 {
        let _ts = env.ledger().timestamp(); // Should Warn
        i += 1;
    }
}

fn bad_sequence_in_loop_loop(env: Env) {
    loop {
        let _seq = env.ledger().sequence(); // Should Warn
        break;
    }
}

fn bad_network_id_in_closure(env: Env) {
    (0..5).for_each(|_| {
        let _nid = env.ledger().network_id(); // Should Warn
    });
}

fn bad_all_accessors_in_loop(env: Env) {
    for _ in 0..3 {
        let _seq = env.ledger().sequence(); // Should Warn
        let _ts = env.ledger().timestamp(); // Should Warn
        let _nid = env.ledger().network_id(); // Should Warn
    }
}

// --- Triggering case: the receiver is a BOUND ledger context (#795) ---
//
// `is_ledger_receiver` used to require the receiver expression to be the
// literal env.ledger() call, so a `let ledger = env.ledger();` binding fed
// every accessor read in the loop without ever tripping the lint. The
// binding hop is now resolved.

fn bad_bound_ledger_timestamp_in_loop(env: Env) {
    let ledger = env.ledger(); // binding, not a per-iteration call
    for _ in 0..10 {
        let _ts = ledger.timestamp(); // Should Warn - bound receiver in loop
    }
}

fn bad_bound_ledger_sequence_in_loop(env: Env) {
    let ledger = env.ledger();
    for _ in 0..10 {
        let _seq = ledger.sequence(); // Should Warn - second accessor, second diagnostic
    }
}

// --- Non-triggering: a user type whose printed name merely CONTAINS `Env`.
// The old substring test on the printed type made any local Environment/
// EnvWrapper satisfy the Env check (#795); only soroban_sdk::Env does now.

pub struct Environment;

impl Environment {
    pub fn ledger(&self) -> EnvironmentLedger {
        EnvironmentLedger
    }
}

pub struct EnvironmentLedger;

impl EnvironmentLedger {
    pub fn timestamp(&self) -> u64 {
        0
    }
}

fn good_environment_namesake_in_loop() {
    let environment = Environment;
    for _ in 0..10 {
        let _ts = environment.ledger().timestamp(); // Good - not soroban_sdk::Env
    }
}

// --- Non-triggering cases (ledger reads outside loops) ---

fn good_sequence_outside_loop(env: Env) {
    let _seq = env.ledger().sequence(); // Good — outside any loop
}

fn good_timestamp_outside_loop(env: Env) {
    let _ts = env.ledger().timestamp(); // Good — outside any loop
}

fn good_network_id_outside_loop(env: Env) {
    let _nid = env.ledger().network_id(); // Good — outside any loop
}

fn good_sequence_hoisted(env: Env) {
    let seq = env.ledger().sequence(); // Good — hoisted outside the loop
    for _ in 0..10 {
        let _ = seq;
    }
}

#[allow(ledger_context_read_in_loop)]
fn allowed_sequence_in_loop(env: Env) {
    for _ in 0..10 {
        let _seq = env.ledger().sequence(); // Good (allowed)
    }
}

fn main() {}
