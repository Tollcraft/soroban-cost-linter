//! #1032: one diagnostic per `from_bytes` expression.
//!
//! `string_concat_in_loop` is warn-by-default, so this file needs no lint
//! attribute — the golden pins the "on by default" note the same way
//! `ui/string_concat_in_loop.stderr` does.
//!
//! The two reporting branches (`append` and `+`) were collapsed into a single
//! `span_lint_and_help` call site, so the invariant "one expression, one
//! warning" is now structural rather than held by a `return`. These three
//! loops pin it: three growth calls feeding one `from_bytes` still report
//! once, two `from_bytes` expressions report twice, and a buffer declared
//! inside the loop reports not at all.
//!
//! Two sibling lints also fire on the `append`/`push` growth calls themselves
//! (`bytes_append_in_loop`, `soroban_inefficient_bytes_concat`). They are
//! allowed here so the golden pins only `string_concat_in_loop`, which is the
//! lint #1032 is about; their own fixtures live elsewhere under `ui/`.

#![allow(bytes_append_in_loop, soroban_inefficient_bytes_concat)]

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
            pub fn push(&mut self, byte: u8) {
                self.0.push(byte);
            }
            // Returns `&mut Self` so one expression can chain two growth
            // calls, which is the shape that used to be diagnosed twice.
            pub fn append(&mut self, other: &mut Bytes) -> &mut Bytes {
                self.0.extend_from_slice(&other.0);
                self
            }
            pub fn as_slice(&self) -> &[u8] {
                &self.0
            }
        }

        impl core::ops::AddAssign for Bytes {
            fn add_assign(&mut self, rhs: Bytes) {
                self.0.extend_from_slice(&rhs.0);
            }
        }
    }

    pub use self::bytes::Bytes;
    pub use self::string::String;
}

use soroban_sdk::{Bytes, Env, String};

// Positive (bad): a chained `append` and a `+=` feed ONE `from_bytes`.
// Three growth calls, one construction expression, one warning.
fn one_warning_per_expression(env: Env) {
    let mut bytes = Bytes::new();
    let tail = [1u8, 2, 3];
    for byte in 0..10u8 {
        let mut extra = Bytes::new();
        extra.push(byte);
        bytes.append(&mut extra).append(&mut extra);
        bytes += Bytes::from_slice(&tail);
        let _ = String::from_bytes(&env, bytes.as_slice()); // Should Warn once
    }
}

// Positive (bad): two construction expressions in one loop body.
// The count follows the expressions, never the growth calls: exactly two.
fn two_expressions_two_warnings(env: Env) {
    let mut bytes = Bytes::new();
    for byte in 0..10u8 {
        bytes.push(byte);
        let _ = String::from_bytes(&env, bytes.as_slice()); // Should Warn
        let mut extra = Bytes::from_slice(&[byte]);
        bytes.append(&mut extra);
        let _ = String::from_bytes(&env, bytes.as_slice()); // Should Warn
    }
}

// Negative (good): the buffer is re-declared inside the loop, so nothing
// carries across iterations and there is nothing to report.
fn good_buffer_declared_in_loop(env: Env) {
    for byte in 0..10u8 {
        let mut bytes = Bytes::new();
        bytes.push(byte);
        let _ = String::from_bytes(&env, bytes.as_slice()); // Good
    }
}

fn main() {}
