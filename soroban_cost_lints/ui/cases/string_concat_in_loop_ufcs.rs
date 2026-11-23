//! #1031: path spellings of `String::from_bytes` inside a loop.
//!
//! `string_concat_in_loop` matches the call by its resolved definition, not by
//! how the callee is spelled. Soroban SDK 28 exposes no `String::append`, so the
//! lint's only target is `String::from_bytes` rebuilt from a growing buffer, and
//! that call can be written as the imported path, the fully qualified path, or
//! the type-relative `<String>::from_bytes` form. All three resolve to the same
//! function and must each report once; a call after the loop stays silent.

pub mod soroban_sdk {
    pub struct Env;

    pub mod string {
        use super::Env;

        pub struct String;
        impl String {
            pub fn from_bytes(_env: &Env, _bytes: &[u8]) -> String {
                String
            }
        }
    }

    pub use self::string::String;
}

use soroban_sdk::{Env, String};

// Positive (bad): three spellings of the same call, one growing buffer.
fn path_spellings_in_one_loop(env: Env) {
    let mut bytes = std::vec::Vec::new();
    for byte in 0..10u8 {
        bytes.push(byte);
        let _ = String::from_bytes(&env, &bytes); // Should Warn — imported path
        let _ = soroban_sdk::String::from_bytes(&env, &bytes); // Should Warn — fully qualified
        let _ = <String>::from_bytes(&env, &bytes); // Should Warn — type-relative
    }
}

// Negative (good): the type-relative spelling once, after the loop.
fn type_relative_after_loop(env: Env) {
    let mut bytes = std::vec::Vec::new();
    for byte in 0..10u8 {
        bytes.push(byte);
    }
    let _ = <String>::from_bytes(&env, &bytes); // Should not warn
}

fn main() {}
