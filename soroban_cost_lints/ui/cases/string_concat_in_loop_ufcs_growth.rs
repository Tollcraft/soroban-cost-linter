//! #1031: the buffer grows through a path (UFCS) call inside the loop.
//!
//! `Vec::append(&mut bytes, &mut piece)` runs the same associated function as
//! `bytes.append(&mut piece)`, so a `String::from_bytes` rebuilt from that
//! buffer in the same loop has the same quadratic cost and must report once,
//! spanned on the `from_bytes` call. The growth call is matched by its
//! resolved definition, so trait-qualified spellings count too, and a
//! path-spelled `Vec::clear` resets the buffer like `bytes.clear()` does.

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

// Positive (bad): `Vec::append` through its path, as the issue describes.
fn ufcs_append(env: Env) {
    let mut bytes: Vec<u8> = Vec::new();
    for byte in 0..10u8 {
        let mut piece = vec![byte];
        Vec::append(&mut bytes, &mut piece);
        let _ = String::from_bytes(&env, &bytes); // Should Warn — Vec::append
    }
}

// Positive (bad): `push` and `extend_from_slice` through their paths.
fn ufcs_push_and_extend_from_slice(env: Env) {
    let mut bytes: Vec<u8> = Vec::new();
    for byte in 0..10u8 {
        Vec::push(&mut bytes, byte);
        std::vec::Vec::extend_from_slice(&mut bytes, &[byte]);
        let _ = String::from_bytes(&env, &bytes); // Should Warn — Vec::push, Vec::extend_from_slice
    }
}

// Positive (bad): a trait-qualified spelling resolves to the same method name.
fn trait_qualified_extend(env: Env) {
    let mut bytes: Vec<u8> = Vec::new();
    for byte in 0..10u8 {
        <Vec<u8> as Extend<u8>>::extend(&mut bytes, [byte]);
        let _ = String::from_bytes(&env, &bytes); // Should Warn — <Vec<u8> as Extend<u8>>::extend
    }
}

// Negative (good): the path-spelled clear resets the buffer every iteration.
fn ufcs_clear_resets(env: Env) {
    let mut bytes: Vec<u8> = Vec::new();
    for byte in 0..10u8 {
        Vec::clear(&mut bytes);
        Vec::push(&mut bytes, byte);
        let _ = String::from_bytes(&env, &bytes); // Should not warn — cleared first
    }
}

// Negative (good): a path call that grows a different buffer.
fn ufcs_growth_of_another_buffer(env: Env) {
    let bytes: Vec<u8> = vec![1, 2, 3];
    let mut other: Vec<u8> = Vec::new();
    for byte in 0..10u8 {
        Vec::push(&mut other, byte);
        let _ = String::from_bytes(&env, &bytes); // Should not warn — `bytes` never grows
    }
}

// Negative (good): a free function named `push` is not the buffer's method.
mod helpers {
    pub fn push(_buf: &mut Vec<u8>, _byte: u8) {}
}

fn free_function_named_push(env: Env) {
    let mut bytes: Vec<u8> = Vec::new();
    for byte in 0..10u8 {
        helpers::push(&mut bytes, byte);
        let _ = String::from_bytes(&env, &bytes); // Should not warn — not an associated fn
    }
}

fn main() {}
