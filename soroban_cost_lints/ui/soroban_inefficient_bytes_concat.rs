//! UI fixtures for repeated concatenation of Soroban byte buffers.
//!
//! The negative example keeps appending inside an unbounded-cost loop so the
//! lint can catch the repeated allocation pattern. The allowed example shows
//! how a caller can intentionally suppress the warning for a tiny bounded
//! loop where the simpler code is preferable.

#![warn(soroban_inefficient_bytes_concat)]

//! UI fixtures for the inefficient Soroban `Bytes` concatenation lint.
//!
//! The positive case deliberately grows a host-backed value in a loop. The
//! negative case keeps the small loop as a regression fixture for the explicit
//! lint allowance, so this file documents both the diagnostic and its escape
//! hatch without changing the expected warning output.

pub mod soroban_sdk {
    pub struct Bytes;
    impl Bytes {
        /// Appends one value to the fixture byte buffer.
        pub fn push_back(&mut self, _val: u32) {}
        /// Appends another fixture byte buffer.
        pub fn append(&mut self, _other: &Bytes) {}
    }
}
use soroban_sdk::Bytes;

fn bad_push_back(mut b: Bytes) {
    // Repeated growth in the loop can repeatedly copy the existing buffer.
    for _ in 0..10 {
        b.push_back(1); //~ WARNING inefficient Bytes concatenation inside a loop
    }
}

#[allow(soroban_inefficient_bytes_concat)]
fn good_small_push_back(mut b: Bytes) {
    // This remains an allowed fixture because callers may intentionally accept
    // the bounded cost for a very small, fixed-size operation.
    // False positive: loop is small and provably bounded, so cost is negligible,
    // but lint flags it anyway unless allowed.
    for _ in 0..2 {
        b.push_back(1);
    }
}

fn main() {}
