#![warn(bytes_append_in_loop)]

pub mod soroban_sdk {
    pub struct Bytes;
    impl Bytes {
        pub fn append(&mut self, _other: &Bytes) {}
    }
}
use soroban_sdk::Bytes;

fn bad_append(mut b: Bytes, other: &Bytes) {
    for _ in 0..100 {
        b.append(other); //~ WARNING repeatedly growing SDK container inside a loop
    }
}

#[allow(bytes_append_in_loop)]
fn good_small_append(mut b: Bytes, other: &Bytes) {
    // False positive: bounded small loop
    for _ in 0..2 {
        b.append(other);
    }
}

fn main() {}
