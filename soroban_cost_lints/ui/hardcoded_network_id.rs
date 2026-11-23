#![warn(hardcoded_network_id)]

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
            pub fn network_id(&self) -> [u8; 32] {
                [0u8; 32]
            }
        }
    }
}

use soroban_sdk::Env;

const TESTNET_ID: [u8; 32] = [0u8; 32];

fn bad_compare_array_literal(env: Env) {
    if env.ledger().network_id() == [0u8; 32] {
    }
}

fn bad_compare_const_id(env: Env) {
    if env.ledger().network_id() == TESTNET_ID {
    }
}

fn bad_compare_byte_string(env: Env) {
    if env.ledger().network_id() == *b"01234567890123456789012345678901" {
    }
}

fn bad_passphrase_literal() {
    let _ = "Test SDF Network ; September 2015";
}

fn good_bare_read(env: Env) -> [u8; 32] {
    env.ledger().network_id()
}

fn good_compare_dynamic(env: Env, expected: [u8; 32]) -> bool {
    env.ledger().network_id() == expected
}

#[allow(hardcoded_network_id)]
fn allowed_hardcoded_id(env: Env) {
    if env.ledger().network_id() == [0u8; 32] {
    }
}

fn main() {}
