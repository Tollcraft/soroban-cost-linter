// UI test fixture for `excessive_vec_capacity`.
//
// Compile with the soroban_cost_lints Dylint library to verify that the
// lint fires (or does not fire) as expected.

pub mod soroban_sdk {
    pub mod vec {
        pub struct Vec;
        impl Vec {
            pub fn new() -> Vec { Vec }
            pub fn with_capacity(_n: u32) -> Vec { Vec }
            pub fn push_back(&mut self, _v: i32) {}
            pub fn reserve(&mut self, _additional: u32) {}
        }
    }
}

use soroban_sdk::vec::Vec;

// Should Warn — wildly excessive capacity (above threshold)
fn bad_with_capacity() {
    let _v = Vec::with_capacity(1_000_000); //~ ERROR excessive pre-allocation capacity in Soroban Vec
}

// Should Warn — excessive reserve (above threshold)
fn bad_reserve() {
    let mut v = Vec::new();
    v.reserve(500_000); //~ ERROR excessive pre-allocation capacity in Soroban Vec
}

// Good — below threshold, no warning
fn good_small_capacity() {
    let _v = Vec::with_capacity(100);
}

// Good — runtime-derived capacity, no warning
fn good_runtime_capacity(n: u32) {
    let _v = Vec::with_capacity(n);
}

// Good — exactly at threshold, no warning
fn good_exact_threshold() {
    let _v = Vec::with_capacity(4096);
}

// Good — runtime-derived reserve, no warning
fn good_runtime_reserve(n: u32) {
    let mut v = Vec::new();
    v.reserve(n);
}

fn main() {}
