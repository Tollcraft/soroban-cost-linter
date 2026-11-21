#![warn(bytes_indexed_read_in_loop)]

pub mod soroban_sdk {
    pub struct Bytes;
    impl Bytes {
        pub fn get(&self, _index: u32) -> Option<u8> {
            Some(0)
        }
    }

    pub struct Vec;
    impl Vec {
        pub fn get(&self, _index: u32) -> Option<u8> {
            Some(0)
        }
    }

    pub struct Map;
    impl Map {
        pub fn get(&self, _key: u32) -> Option<u8> {
            Some(0)
        }
    }
}

use soroban_sdk::{Bytes, Map, Vec};

fn bad_for(bytes: &Bytes) {
    for i in 0..2 {
        let _ = bytes.get(i).unwrap(); //~ WARNING per-byte Bytes::get call inside a loop
    }
}

fn bad_while(bytes: &Bytes) {
    let mut i = 0;
    while i < 2 {
        let _ = bytes.get(i).unwrap(); //~ WARNING per-byte Bytes::get call inside a loop
        i += 1;
    }
}

fn bad_loop(bytes: &Bytes) {
    let mut i = 0;
    loop {
        let _ = bytes.get(i).unwrap(); //~ WARNING per-byte Bytes::get call inside a loop
        i += 1;
        if i == 2 {
            break;
        }
    }
}

fn good_other_types(vec: &Vec, map: &Map) {
    for i in 0..2 {
        let _ = vec.get(i);
        let _ = map.get(i);
    }
}

fn good_outside_loop(bytes: &Bytes) {
    let _ = bytes.get(0);
}

#[allow(bytes_indexed_read_in_loop)]
fn allowed(bytes: &Bytes) {
    for i in 0..2 {
        let _ = bytes.get(i);
    }
}

fn main() {}
