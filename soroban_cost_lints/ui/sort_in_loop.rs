#![warn(sort_in_loop)]

pub mod soroban_sdk {
    use std::cmp::Ordering;

    pub struct Vec;
    impl Vec {
        pub fn sort(&mut self) {}

        pub fn sort_by<F>(&mut self, _compare: F)
        where
            F: FnMut(&u32, &u32) -> Ordering,
        {
        }
    }
}

use soroban_sdk::Vec;

fn bad_for(mut values: Vec) {
    for _ in 0..2 {
        values.sort(); //~ WARNING sorting a Soroban Vec inside a loop
    }
}

fn bad_while(mut values: Vec) {
    let mut i = 0;
    while i < 2 {
        values.sort_by(u32::cmp); //~ WARNING sorting a Soroban Vec inside a loop
        i += 1;
    }
}

fn bad_loop(mut values: Vec) {
    loop {
        values.sort(); //~ WARNING sorting a Soroban Vec inside a loop
        break;
    }
}

fn good_outside_loop(mut values: Vec) {
    values.sort();
    values.sort_by(u32::cmp);
}

fn good_native_slice(values: &mut [u32]) {
    for _ in 0..2 {
        values.sort();
        values.sort_by(u32::cmp);
    }
}

fn main() {}
