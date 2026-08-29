#![warn(collection_len_in_loop_condition)]
#![allow(clippy::all)]
#![allow(unused)]

pub mod soroban_sdk {
    pub struct Env;
    pub mod vec {
        pub struct Vec<T>(std::marker::PhantomData<T>);
        impl<T> Vec<T> {
            pub fn new(_env: &super::Env) -> Self { Self(std::marker::PhantomData) }
            pub fn len(&self) -> u32 { 0 }
            pub fn get(&self, _i: u32) -> T { unimplemented!() }
            pub fn push_back(&mut self, _item: T) {}
        }
    }
    pub use self::vec::Vec;
    
    pub mod map {
        pub struct Map<K, V>(std::marker::PhantomData<(K, V)>);
        impl<K, V> Map<K, V> {
            pub fn len(&self) -> u32 { 0 }
        }
    }
    pub use self::map::Map;
    
    pub mod bytes {
        pub struct Bytes;
        impl Bytes {
            pub fn len(&self) -> u32 { 0 }
        }
    }
    pub use self::bytes::Bytes;
}

use soroban_sdk::{Env, Vec, Map, Bytes};

pub fn firing_case(env: Env, vec: Vec<u32>) {
    let mut i = 0;
    while i < vec.len() {
        let _ = vec.get(i);
        i += 1;
    }
}

pub fn non_firing_case_mutated(env: Env, mut vec: Vec<u32>) {
    let mut i = 0;
    // #[allow(collection_len_in_loop_condition)] // Should not fire because of push_back
    while i < vec.len() {
        vec.push_back(i);
        i += 1;
    }
}

pub fn non_firing_case_for_loop(env: Env, vec: Vec<u32>) {
    for _ in 0..vec.len() {
        let _ = vec.get(0);
    }
}

pub fn non_firing_case_std_vec(env: Env, vec: std::vec::Vec<u32>) {
    let mut i = 0;
    while (i as usize) < vec.len() {
        let _ = vec.get(0);
        i += 1;
    }
}

fn main() {}
