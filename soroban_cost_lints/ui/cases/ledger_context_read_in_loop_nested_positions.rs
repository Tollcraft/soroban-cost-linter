#![allow(unnecessary_host_function_call)]

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
            pub fn sequence(&self) -> u32 {
                0
            }
            pub fn timestamp(&self) -> u64 {
                0
            }
            pub fn network_id(&self) -> [u8; 32] {
                [0u8; 32]
            }
        }
    }
}

use soroban_sdk::Env;

fn reads_in_match_arms(env: Env, selector: u32) {
    for _ in 0..3 {
        match selector {
            0 => {
                let _sequence = env.ledger().sequence();
            }
            _ => {
                let _timestamp = env.ledger().timestamp();
            }
        }
    }
}

fn read_in_nested_closure(env: Env) {
    (0..3).for_each(|_| {
        (0..3).for_each(|_| {
            let _network_id = env.ledger().network_id();
        });
    });
}

fn read_outside_loop_is_silent(env: Env) {
    let _sequence = env.ledger().sequence();
}

fn main() {}
