pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env {
        fn clone(&self) -> Self { Env }
    }

    // Minimal `Bytes` stub: just enough surface for the slice/copy methods
    // this lint targets. `slice` returns a fresh `Bytes` (a copy); the
    // copy-range methods move bytes between a `Bytes` and a native slice.
    pub struct Bytes(pub std::vec::Vec<u8>);
    impl Bytes {
        pub fn from(_s: &str) -> Bytes { Bytes(vec![]) }
        pub fn slice(&self, _start: u32, _end: u32) -> Bytes { Bytes(vec![]) }
        pub fn copy_from_slice(&mut self, _slice: &[u8]) {}
        pub fn copy_to_slice(&self, _slice: &mut [u8]) {}
    }
}

use soroban_sdk::Env;
use soroban_sdk::Bytes;

// =======================================================================
// bytes_slice_copy_in_loop — Fixtures
// =======================================================================

// Parser that walks a caller-supplied payload four bytes at a time. Each
// iteration takes a fresh sub-slice of the remaining buffer, which copies an
// average of half the remaining bytes — quadratic overall. Should Warn.
fn bad_slice_in_while_loop(payload: Bytes) {
    let mut i = 0u32;
    while i + 4 <= 64 {
        let _chunk = payload.slice(i, i + 4); // Should Warn
        i += 4;
    }
}

// `for` loops trip the same detection. Should Warn.
fn bad_slice_in_for_loop(payload: Bytes, n: u32) {
    for i in 0..n {
        let _chunk = payload.slice(i, i + 4); // Should Warn
    }
}

// Copying the whole buffer out into a native slice on every iteration also
// re-copies the buffer each pass. Should Warn.
fn bad_copy_to_slice_in_loop(payload: Bytes, n: u32) {
    for _ in 0..n {
        let mut buf = [0u8; 8];
        payload.copy_to_slice(&mut buf); // Should Warn
    }
}

// Copying a (bounded) native slice into the buffer per iteration. Should Warn.
fn bad_copy_from_slice_in_loop(n: u32) {
    let mut buf = Bytes(vec![]);
    for _ in 0..n {
        buf.copy_from_slice(&[0u8, 1, 2, 3]); // Should Warn
    }
}

// A single slice call that is not inside a loop must not fire. Should NOT warn.
fn good_single_slice_outside_loop(payload: Bytes) -> Bytes {
    payload.slice(0, 4) // Good — not in a loop
}

// Slicing once outside the loop (a single copy) and then working with the
// extracted value avoids the per-iteration copy. Should NOT warn.
fn good_slice_once_outside_loop(payload: Bytes) -> Bytes {
    let whole = payload.slice(0, 64); // outside loop — single copy
    whole
}

#[allow(bytes_slice_copy_in_loop)]
fn allowed_slice_in_loop(payload: Bytes) {
    for i in 0..10 {
        let _ = payload.slice(i, i + 1); // Good (allowed)
    }
}

fn main() {}
