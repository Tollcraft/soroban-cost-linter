#![warn(symbol_to_string_in_loop)]
#![allow(clippy::all)]
#![allow(unused)]

pub mod soroban_sdk {
    pub struct Symbol;
    impl Symbol {
        pub fn to_string(&self) -> String {
            String
        }
    }

    pub struct String;
    impl String {
        pub fn to_string(&self) -> String {
            String
        }
        pub fn len(&self) -> usize {
            0
        }
    }
}

use soroban_sdk::{String, Symbol};

fn bad_for(symbol: Symbol) {
    for _ in 0..3 {
        let _text = symbol.to_string(); //~ WARNING Symbol::to_string called inside a loop
    }
}

fn bad_while(symbol: Symbol) {
    let mut i = 0;
    while i < 3 {
        let _text = symbol.to_string(); //~ WARNING Symbol::to_string called inside a loop
        i += 1;
    }
}

fn bad_loop(symbol: Symbol) {
    let mut i = 0;
    loop {
        let _text = symbol.to_string(); //~ WARNING Symbol::to_string called inside a loop
        i += 1;
        if i == 3 {
            break;
        }
    }
}

fn good_outside_loop(symbol: Symbol) {
    let _text = symbol.to_string();
}

fn good_string_receiver(text: String) {
    for _ in 0..3 {
        let _copy = text.to_string();
    }
}

fn good_native_receiver() {
    let number = 7i32;
    for _ in 0..3 {
        let _text = number.to_string();
    }
}

fn good_built_once(symbol: Symbol) {
    let text = symbol.to_string();
    for _ in 0..3 {
        let _length = text.len();
    }
}

fn good_uncalled_closure(symbol: Symbol) {
    let _make_text = || symbol.to_string();
}

#[allow(symbol_to_string_in_loop)]
fn allowed_conversion(symbol: Symbol) {
    for _ in 0..3 {
        let _text = symbol.to_string();
    }
}

fn main() {}
