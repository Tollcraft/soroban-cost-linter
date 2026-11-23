#![warn(bytes_append_in_loop)]
#![allow(soroban_inefficient_bytes_concat)]

pub mod soroban_sdk {
    pub struct Bytes;
    impl Bytes {
        pub fn append(&mut self, _other: &Bytes) {}
        pub fn push_back(&mut self, _byte: u8) {}
        pub fn insert(&mut self, _i: u32, _byte: u8) {}
        pub fn extend_from_array<const N: usize>(&mut self, _arr: [u8; N]) {}
        pub fn extend_from_slice(&mut self, _slice: &[u8]) {}
    }

    pub struct Vec<T>(std::marker::PhantomData<T>);
    impl<T> Vec<T> {
        pub fn new() -> Self {
            Self(std::marker::PhantomData)
        }
        pub fn append(&mut self, _other: &Vec<T>) {}
        pub fn push_back(&mut self, _item: T) {}
        pub fn push_front(&mut self, _item: T) {}
        pub fn insert(&mut self, _i: u32, _item: T) {}
        pub fn extend_from_array<const N: usize>(&mut self, _arr: [T; N]) {}
        pub fn extend_from_slice(&mut self, _slice: &[T]) {}
        pub fn concat(&self, _other: &Vec<T>) -> Vec<T> {
            Vec::new()
        }
    }

    pub struct Map<K, V>(std::marker::PhantomData<(K, V)>);
    impl<K, V> Map<K, V> {
        pub fn insert(&mut self, _key: K, _val: V) {}
    }
}
use soroban_sdk::{Bytes, Map, Vec};

fn bad_bytes_append(mut b: Bytes, other: &Bytes) {
    for _ in 0..100 {
        b.append(other); //~ WARNING repeatedly growing SDK container inside a loop
    }
}

fn bad_bytes_push_back(mut b: Bytes) {
    for i in 0..100 {
        b.push_back(i as u8); //~ WARNING repeatedly growing SDK container inside a loop
    }
}

fn bad_bytes_insert(mut b: Bytes) {
    for i in 0..100 {
        b.insert(0, i as u8); //~ WARNING repeatedly growing SDK container inside a loop
    }
}

fn bad_bytes_extend_from_array(mut b: Bytes) {
    for _ in 0..100 {
        b.extend_from_array([1, 2, 3]); //~ WARNING repeatedly growing SDK container inside a loop
    }
}

fn bad_bytes_extend_from_slice(mut out: Bytes, chunk: &[u8]) {
    for _ in 0..100 {
        out.extend_from_slice(chunk); //~ WARNING repeatedly growing SDK container inside a loop
    }
}

fn bad_vec_append(mut v: Vec<u32>, other: &Vec<u32>) {
    for _ in 0..100 {
        v.append(other); //~ WARNING repeatedly growing SDK container inside a loop
    }
}

fn bad_vec_push_back(mut v: Vec<u32>) {
    for i in 0..100 {
        v.push_back(i); //~ WARNING repeatedly growing SDK container inside a loop
    }
}

fn bad_vec_push_front(mut v: Vec<u32>) {
    for i in 0..100 {
        v.push_front(i); //~ WARNING repeatedly growing SDK container inside a loop
    }
}

fn bad_vec_insert(mut v: Vec<u32>) {
    for i in 0..100 {
        v.insert(0, i); //~ WARNING repeatedly growing SDK container inside a loop
    }
}

fn bad_vec_extend_from_array(mut v: Vec<u32>) {
    for _ in 0..100 {
        v.extend_from_array([1, 2, 3]); //~ WARNING repeatedly growing SDK container inside a loop
    }
}

fn bad_vec_extend_from_slice(mut out: Vec<u32>, chunk: &[u32]) {
    for _ in 0..100 {
        out.extend_from_slice(chunk); //~ WARNING repeatedly growing SDK container inside a loop
    }
}

fn bad_vec_concat(a: Vec<u32>, b: &Vec<u32>) {
    for _ in 0..100 {
        let _ = a.concat(b); //~ WARNING repeatedly growing SDK container inside a loop
    }
}

fn bad_map_insert(mut m: Map<u32, u32>) {
    for i in 0..100 {
        m.insert(i, i); //~ WARNING repeatedly growing SDK container inside a loop
    }
}

fn good_growth_outside_loop(mut b: Bytes, other: &Bytes, mut v: Vec<u32>, mut m: Map<u32, u32>) {
    b.append(other);
    b.push_back(1);
    b.insert(0, 1);
    b.extend_from_array([1, 2]);
    b.extend_from_slice(&[1, 2]);
    v.append(&Vec::new());
    v.push_back(1);
    v.push_front(2);
    v.insert(0, 3);
    v.extend_from_array([1, 2]);
    v.extend_from_slice(&[1, 2]);
    let _ = v.concat(&Vec::new());
    m.insert(1, 2);
}

fn good_native_vec_in_loop() {
    let mut native_vec = std::vec::Vec::new();
    for i in 0..100 {
        native_vec.push(i);
        native_vec.extend_from_slice(&[1, 2, 3]);
    }
}

#[allow(bytes_append_in_loop)]
fn good_small_append(mut b: Bytes, other: &Bytes) {
    // False positive: bounded small loop
    for _ in 0..2 {
        b.append(other);
    }
}

fn main() {}
