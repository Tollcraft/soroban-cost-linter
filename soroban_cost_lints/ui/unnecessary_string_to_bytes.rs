#![warn(unnecessary_string_to_bytes)]

pub mod soroban_sdk {
    pub struct String;
    impl String {
        pub fn to_bytes(&self) -> Bytes {
            Bytes
        }
    }
    pub struct Bytes;
}
use soroban_sdk::String;

fn bad_to_bytes(s: String) {
    let _ = s.to_bytes(); //~ WARNING unnecessary String to Bytes conversion
}

#[allow(unnecessary_string_to_bytes)]
fn good_required_to_bytes(s: String) {
    // False positive: when an external trait strictly requires Bytes and cannot be changed
    takes_bytes(s.to_bytes());
}

fn takes_bytes(_b: soroban_sdk::Bytes) {}

fn main() {}
