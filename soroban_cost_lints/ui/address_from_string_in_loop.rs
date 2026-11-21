#![warn(address_from_string_in_loop)]

pub mod soroban_sdk {
    pub struct Env;
    pub struct String;
    pub struct Address;

    impl String {
        pub fn from_str(_env: &Env, _value: &str) -> Result<Self, ()> {
            Ok(Self)
        }
    }

    impl Address {
        pub fn from_string(_env: &Env, _value: &String) -> Result<Self, ()> {
            Ok(Self)
        }
    }
}

use soroban_sdk::{Address, Env, String};

fn bad_for(env: &Env, strkey: &String) {
    for _ in 0..2 {
        let _ = Address::from_string(env, strkey); //~ WARNING Address::from_string called inside a loop
    }
}

fn bad_while(env: &Env, strkey: &String) {
    let mut i = 0;
    while i < 2 {
        let _ = Address::from_string(env, strkey); //~ WARNING Address::from_string called inside a loop
        i += 1;
    }
}

fn bad_loop(env: &Env, strkey: &String) {
    loop {
        let _ = Address::from_string(env, strkey); //~ WARNING Address::from_string called inside a loop
        break;
    }
}

fn good_hoisted(env: &Env, strkey: &String) {
    let address = Address::from_string(env, strkey);
    for _ in 0..2 {
        let _ = &address;
    }
}

fn good_outside_loop(env: &Env, strkey: &String) {
    let _ = Address::from_string(env, strkey);
}

fn good_string_parse(env: &Env, strkey: &str) {
    for _ in 0..2 {
        let _ = String::from_str(env, strkey);
    }
}

fn main() {}
