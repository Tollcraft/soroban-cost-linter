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
        }
    }
}

use soroban_sdk::Env;

// =======================================================================
// unnecessary_host_function_call — Fixtures
// =======================================================================

fn bad_host_call_in_for_loop(env: Env) {
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

fn bad_host_call_in_nested_loop(env: Env) {
    for _ in 0..5 {
        for _ in 0..5 {
            let _seq = env.ledger().sequence(); // Should Warn
        }
    }
}

fn good_host_call_outside_loop(env: Env) {
    let seq = env.ledger().sequence(); // Good — called once before the loop
    for _ in 0..10 {
        let _seq = seq;
    }
}

fn bad_host_call_multiple_times_in_loop(env: Env) {
    for _ in 0..10 {
        let _seq1 = env.ledger().sequence(); // Should Warn
        let _seq2 = env.ledger().sequence(); // Should Warn
    }
}

#[allow(unnecessary_host_function_call)]
fn allowed_host_call_in_loop(env: Env) {
    for _ in 0..10 {
        let _seq = env.ledger().sequence(); // Good (allowed)
    }
}

fn main() {}
