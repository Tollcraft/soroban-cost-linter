#![allow(dead_code, unused_variables)]

pub mod soroban_sdk {
    pub mod vec {
        use std::marker::PhantomData;
        pub struct Vec<T>(PhantomData<T>);
        impl<T> Vec<T> {
            pub fn new() -> Self { Vec(PhantomData) }
            pub fn len(&self) -> u32 { 0 }
            pub fn get(&self, index: u32) -> Option<T> { None }
            pub fn get_unchecked(&self, index: u32) -> T { panic!() }
            pub fn push_back(&mut self, item: T) {}
        }
    }
    pub use self::vec::Vec;
}

use soroban_sdk::Vec;

// Positive (bad): loop over range, index by loop variable
fn trigger_get(v: Vec<u32>) {
    for i in 0..v.len() {
        let _ = v.get(i);
    }
}

fn trigger_get_unchecked(v: Vec<u32>) {
    for i in 0..v.len() {
        let _ = v.get_unchecked(i);
    }
}

// Positive (bad): loop over range with cast index
fn trigger_cast(v: Vec<u32>) {
    for i in 0..10 {
        let _ = v.get(i as u32);
    }
}

// Negative (good): index by something other than loop variable
fn good_random_access(v: Vec<u32>, idx: u32) {
    for i in 0..10 {
        let _ = v.get(idx);
    }
}

// Negative (good): native rust Vec
fn good_native_vec(v: std::vec::Vec<u32>) {
    for i in 0..v.len() {
        let _ = v.get(i);
    }
}

// Negative (good): loop mutates the collection
fn good_mutated(mut v: Vec<u32>) {
    for i in 0..v.len() {
        let _ = v.get(i);
        v.push_back(123);
    }
}

fn main() {}
