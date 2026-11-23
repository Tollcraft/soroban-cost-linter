#![warn(string_from_str_in_loop)]
#![allow(clippy::all)]
#![allow(unused)]

pub mod soroban_sdk {
    pub struct Env;

    pub struct String;
    impl String {
        pub fn from_str(_env: &Env, _text: &str) -> String {
            String
        }
    }
}

use soroban_sdk::{Env, String};

struct LocalString;
impl LocalString {
    fn from_str(_env: &Env, _text: &str) -> Self {
        Self
    }
}

const FIXED: &str = "fixed";

fn bad_for(env: &Env) {
    for _ in 0..3 {
        let _value = String::from_str(env, "literal"); //~ WARNING String::from_str with a constant value inside a loop
    }
}

fn bad_while(env: &Env) {
    let mut i = 0;
    while i < 3 {
        let _value = String::from_str(env, FIXED); //~ WARNING String::from_str with a constant value inside a loop
        i += 1;
    }
}

fn bad_loop(env: &Env) {
    loop {
        let _value = String::from_str(env, "literal"); //~ WARNING String::from_str with a constant value inside a loop
        break;
    }
}

fn good_runtime_value(env: &Env, name: &str) {
    for _ in 0..3 {
        let _value = String::from_str(env, name);
    }
}

fn good_outside_loop(env: &Env) {
    let _value = String::from_str(env, "literal");
}

fn good_local_type(env: &Env) {
    for _ in 0..3 {
        let _value = LocalString::from_str(env, "literal");
    }
}

#[allow(string_from_str_in_loop)]
fn allowed(env: &Env) {
    for _ in 0..3 {
        let _value = String::from_str(env, "literal");
    }
}

fn main() {}
