#![warn(inefficient_bytes_concat)]

pub mod soroban_sdk {
    #[derive(Clone, Copy)]
    pub struct Bytes;
}
use soroban_sdk::Bytes;

impl std::ops::Add for Bytes {
    type Output = Bytes;
    fn add(self, _other: Bytes) -> Bytes {
        self
    }
}

fn bad_concat(b1: Bytes, b2: Bytes) {
    for _ in 0..10 {
        let _ = b1 + b2; //~ WARNING inefficient bytes concatenation
    }
}

#[allow(inefficient_bytes_concat)]
fn good_small_concat(b1: Bytes, b2: Bytes) {
    // False positive: loop is bounded to 2 iterations, cost is small
    for _ in 0..2 {
        let _ = b1 + b2;
    }
}

fn main() {}
