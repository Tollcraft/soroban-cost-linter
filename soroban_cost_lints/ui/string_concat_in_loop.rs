pub mod soroban_sdk {
    pub struct Env;
    pub mod string {
        use super::Env;

        pub struct String;
        impl String {
            pub fn from_str(_env: &Env, _value: &str) -> String {
                String
            }
            pub fn from_bytes(_env: &Env, _bytes: &[u8]) -> String {
                String
            }
        }
    }

    pub use self::string::String;
}

use soroban_sdk::{Env, String};

// =======================================================================
// string_concat_in_loop — Fixtures
// =======================================================================

// Positive (bad): reconstructing the String from a growing byte buffer.
fn bad_string_from_bytes_in_loop(env: Env) {
    let mut bytes = std::vec::Vec::new();
    for byte in 0..10 {
        bytes.push(byte);
        let _ = String::from_bytes(&env, &bytes); // Should Warn
    }
}

// Negative (good): construct the String once after the loop.
fn good_string_from_bytes_after_loop(env: Env) {
    let bytes = [1u8, 2, 3];
    for _ in 0..bytes.len() {}
    let _ = String::from_bytes(&env, &bytes); // Good
}

fn good_independent_chunks(env: Env, chunks: &[&[u8]]) {
    for chunk in chunks {
        let _ = String::from_bytes(&env, chunk); // Should not warn
    }
}

fn good_reset_buffer_each_iteration(env: Env, chunks: &[u8]) {
    let mut bytes = std::vec::Vec::new();
    for chunk in chunks {
        bytes.clear();
        bytes.push(*chunk);
        let _ = String::from_bytes(&env, &bytes); // Should not warn
    }
}

fn bad_extend_from_slice(env: Env, chunks: &[&[u8]]) {
    let mut bytes = std::vec::Vec::new();
    for chunk in chunks {
        bytes.extend_from_slice(chunk);
        let _ = String::from_bytes(&env, bytes.as_slice()); // Should Warn
    }
}

fn main() {}
