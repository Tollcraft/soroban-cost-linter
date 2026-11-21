#![warn(address_equality_via_to_string)]

pub mod soroban_sdk {
    #[derive(PartialEq, Eq)]
    pub struct Address;

    #[derive(PartialEq, Eq)]
    pub struct String;

    impl Address {
        pub fn to_string(&self) -> String {
            String
        }
    }
}

use soroban_sdk::{Address, String};

fn address_comparisons(a: &Address, b: &Address) {
    let _ = a.to_string() == b.to_string(); //~ WARNING comparing Address values through their string representations
    let _ = a.to_string() != b.to_string(); //~ WARNING comparing Address values through their string representations
}

fn direct_address_comparison(a: &Address, b: &Address) {
    let _ = a == b;
}

fn string_comparison(s: &String, t: &String) {
    let _ = s == t;
}

fn one_sided_to_string(a: &Address, s: String, t: String) {
    let _ = a.to_string() == s;
    let _ = t == a.to_string();
}

fn main() {}
