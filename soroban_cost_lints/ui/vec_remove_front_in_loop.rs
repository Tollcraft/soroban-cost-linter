#![warn(vec_remove_front_in_loop)]
#![allow(clippy::all)]
#![allow(unused)]

pub mod soroban_sdk {
    pub mod vec {
        pub struct Vec;
        impl Vec {
            pub fn remove(&mut self, _index: u32) {}
        }
    }
}

use soroban_sdk::vec::Vec;

fn bad_for(mut values: Vec) {
    for _ in 0..3 {
        values.remove(0); //~ WARNING Vec::remove(0) inside a loop
    }
}

fn bad_while(mut values: Vec) {
    let mut i = 0;
    while i < 3 {
        values.remove(0); //~ WARNING Vec::remove(0) inside a loop
        i += 1;
    }
}

fn bad_loop(mut values: Vec) {
    loop {
        values.remove(0); //~ WARNING Vec::remove(0) inside a loop
        break;
    }
}

fn good_nonzero_index(mut values: Vec) {
    for i in 0..3 {
        values.remove(i);
    }
}

fn good_outside_loop(mut values: Vec) {
    values.remove(0);
}

fn good_native_vec(mut values: std::vec::Vec<u32>) {
    for _ in 0..3 {
        values.remove(0);
    }
}

#[allow(vec_remove_front_in_loop)]
fn allowed(mut values: Vec) {
    for _ in 0..3 {
        values.remove(0);
    }
}

fn main() {}
