#![warn(host_container_clone_in_loop)]

pub mod soroban_sdk {
    #[derive(Clone)]
    pub struct Vec<T>(core::marker::PhantomData<T>);
    impl<T> Vec<T> {
        pub fn new() -> Self {
            Vec(core::marker::PhantomData)
        }
    }

    #[derive(Clone)]
    pub struct Map<K, V>(core::marker::PhantomData<(K, V)>);
    impl<K, V> Map<K, V> {
        pub fn new() -> Self {
            Map(core::marker::PhantomData)
        }
    }

    #[derive(Clone)]
    pub struct Bytes;
    impl Bytes {
        pub fn new() -> Self {
            Bytes
        }
    }

    #[derive(Clone)]
    pub struct String;
    impl String {
        pub fn new() -> Self {
            String
        }
    }
}

use soroban_sdk::{Bytes, Map, String, Vec};

fn bad_vec_clone_in_for_loop(vec: &Vec<u32>) {
    for _ in 0..10 {
        let _ = vec.clone(); //~ WARNING cloning a host container inside a loop
    }
}

fn bad_map_clone_in_while_loop(map: &Map<u32, u32>) {
    let mut i = 0;
    while i < 10 {
        let _ = map.clone(); //~ WARNING cloning a host container inside a loop
        i += 1;
    }
}

fn bad_bytes_clone_in_loop(bytes: &Bytes) {
    let mut i = 0;
    loop {
        let _ = bytes.clone(); //~ WARNING cloning a host container inside a loop
        i += 1;
        if i >= 10 {
            break;
        }
    }
}

fn bad_string_clone_in_for_loop(s: &String) {
    for _ in 0..10 {
        let _ = s.clone(); //~ WARNING cloning a host container inside a loop
    }
}

fn bad_clone_with_later_use(vec: &Vec<u32>) {
    for _ in 0..10 {
        let cloned = vec.clone(); //~ WARNING cloning a host container inside a loop
        let _use_copy = &cloned;
        let _use_orig = vec;
    }
}

fn good_clone_outside_loop(vec: &Vec<u32>, map: &Map<u32, u32>, bytes: &Bytes, s: &String) {
    let _v = vec.clone();
    let _m = map.clone();
    let _b = bytes.clone();
    let _s = s.clone();
}

fn good_native_vec_clone(native_vec: &std::vec::Vec<u32>) {
    for _ in 0..10 {
        let _ = native_vec.clone();
    }
}

fn good_clone_hoisted_above_loop(vec: &Vec<u32>) {
    let hoisted = vec.clone();
    for _ in 0..10 {
        let _ = &hoisted;
    }
}

#[allow(host_container_clone_in_loop)]
fn allowed_clone_in_loop(vec: &Vec<u32>) {
    for _ in 0..10 {
        let _ = vec.clone();
    }
}

fn main() {}
