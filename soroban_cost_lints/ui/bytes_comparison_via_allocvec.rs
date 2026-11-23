#![warn(bytes_comparison_via_allocvec)]
#![allow(clippy::all)]
#![allow(unused)]

pub mod soroban_sdk {
    #[derive(PartialEq)]
    pub struct Bytes;
    impl Bytes {
        pub fn to_allocvec(&self) -> Vec<u8> {
            Vec::new()
        }
    }
}

use soroban_sdk::Bytes;

fn bad_eq(left: &Bytes, right: &Bytes) {
    let _same = left.to_allocvec() == right.to_allocvec(); //~ WARNING comparing Bytes values through to_allocvec allocates both operands
}

fn bad_ne(left: &Bytes, right: &Bytes) {
    let _different = left.to_allocvec() != right.to_allocvec(); //~ WARNING comparing Bytes values through to_allocvec allocates both operands
}

fn good_direct_comparison(left: &Bytes, right: &Bytes) {
    let _same = left == right;
}

fn good_single_conversion(left: &Bytes, right: Vec<u8>) {
    let _same = left.to_allocvec() == right;
}

fn good_native_vecs(left: Vec<u8>, right: Vec<u8>) {
    let _same = left == right;
}

#[allow(bytes_comparison_via_allocvec)]
fn allowed(left: &Bytes, right: &Bytes) {
    let _same = left.to_allocvec() == right.to_allocvec();
}

fn main() {}
