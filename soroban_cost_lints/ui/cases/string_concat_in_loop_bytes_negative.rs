//! #1033: non-`String` concatenation stays silent under `string_concat_in_loop`.
//!
//! The lint only fires on `soroban_sdk::String::from_bytes`, resolved by def
//! path. Byte-level growth in a loop is owned by the bytes lints, with
//! different remediation advice, so none of the cases below may report here:
//!
//! - `Bytes + Bytes` accumulation in a loop,
//! - an `append`-style call on a non-`String` type in a loop,
//! - a `Vec<u8>` accumulated with `extend_from_slice` in a loop,
//! - a same-named `from_bytes` on a local type, fed by a growing buffer.
//!
//! The last case pins the def-path requirement: if the def-path match were
//! replaced by a check on the name `from_bytes`, it would start warning and
//! this empty golden would fail. The bytes lints are allowed so the empty
//! golden isolates `string_concat_in_loop`.

#![allow(inefficient_bytes_concat, soroban_inefficient_bytes_concat, bytes_append_in_loop)]

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

    pub mod bytes {
        pub struct Bytes(Vec<u8>);

        impl Bytes {
            pub fn new() -> Bytes {
                Bytes(Vec::new())
            }
            pub fn from_slice(value: &[u8]) -> Bytes {
                Bytes(value.to_vec())
            }
            pub fn append(&mut self, other: &Bytes) {
                self.0.extend_from_slice(&other.0);
            }
        }

        impl core::ops::Add for Bytes {
            type Output = Bytes;
            fn add(mut self, rhs: Bytes) -> Bytes {
                self.0.extend_from_slice(&rhs.0);
                self
            }
        }
    }

    pub use self::bytes::Bytes;
    pub use self::string::String;
}

mod local {
    use super::soroban_sdk::Env;

    /// Same associated function name and signature as `soroban_sdk::String`,
    /// but a different definition.
    pub struct Text;
    impl Text {
        pub fn from_bytes(_env: &Env, _bytes: &[u8]) -> Text {
            Text
        }
    }
}

use soroban_sdk::{Bytes, Env};

// Negative: `Bytes + Bytes` in a loop is not a `String` concatenation.
fn bytes_addition_in_loop(chunks: &[&[u8]]) {
    let mut acc = Bytes::new();
    for chunk in chunks {
        acc = acc + Bytes::from_slice(chunk); // Should not warn
    }
    let _ = acc;
}

// Negative: an `append`-style call on a non-`String` receiver in a loop.
fn bytes_append_style_call_in_loop(chunks: &[&[u8]]) {
    let mut acc = Bytes::new();
    for chunk in chunks {
        acc.append(&Bytes::from_slice(chunk)); // Should not warn
    }
    let _ = acc;
}

// Negative: a `Vec<u8>` accumulated in a loop; std `Vec` has no `Add`.
fn vec_extend_from_slice_in_loop(chunks: &[&[u8]]) {
    let mut acc: Vec<u8> = Vec::new();
    for chunk in chunks {
        acc.extend_from_slice(chunk); // Should not warn
    }
    let _ = acc;
}

// Negative: a same-named `from_bytes` on a local type, fed by a growing buffer.
fn local_from_bytes_in_loop(env: Env) {
    let mut bytes = std::vec::Vec::new();
    for byte in 0..10u8 {
        bytes.push(byte);
        let _ = local::Text::from_bytes(&env, &bytes); // Should not warn
    }
}

fn main() {}
