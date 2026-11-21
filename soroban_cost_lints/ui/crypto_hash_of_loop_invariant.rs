#![warn(crypto_hash_of_loop_invariant)]
#![allow(crypto_hash_of_constant)]
#![allow(unnecessary_host_function_call)]
#![allow(clippy::all)]
#![allow(unused)]

pub mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn crypto(&self) -> crypto::Crypto {
            crypto::Crypto
        }
    }

    pub mod crypto {
        pub struct Crypto;
        impl Crypto {
            pub fn sha256(&self, _data: &[u8]) -> [u8; 32] {
                [0; 32]
            }
            pub fn keccak256(&self, _data: &[u8]) -> [u8; 32] {
                [0; 32]
            }
        }
    }
}

use soroban_sdk::Env;

const TAG: &[u8] = b"fixed-tag";

fn bad_sha256(env: Env) {
    let payload = [1u8, 2, 3];
    for _ in 0..3 {
        let _digest = env.crypto().sha256(&payload); //~ WARNING cryptographic hash of a loop-invariant value inside a loop
    }
}

fn bad_keccak256(env: Env) {
    let payload = [4u8, 5, 6];
    let mut i = 0;
    while i < 3 {
        let _digest = env.crypto().keccak256(&payload); //~ WARNING cryptographic hash of a loop-invariant value inside a loop
        i += 1;
    }
}

fn bad_loop(env: Env) {
    let payload = [7u8, 8, 9];
    let mut i = 0;
    loop {
        let _digest = env.crypto().sha256(&payload); //~ WARNING cryptographic hash of a loop-invariant value inside a loop
        i += 1;
        if i == 3 {
            break;
        }
    }
}

fn good_binding_inside_loop(env: Env) {
    for i in 0..3 {
        let payload = [i as u8];
        let _digest = env.crypto().sha256(&payload);
    }
}

fn good_loop_variable_argument(env: Env, values: &[[u8; 3]]) {
    for payload in values {
        let _digest = env.crypto().sha256(payload);
    }
}

fn good_mutated_in_loop(env: Env) {
    let mut payload = [1u8, 2, 3];
    for _ in 0..3 {
        payload[0] += 1;
        let _digest = env.crypto().sha256(&payload);
    }
}

fn good_const_argument_owned_by_other_lint(env: Env) {
    for _ in 0..3 {
        let _digest = env.crypto().sha256(TAG);
    }
}

fn good_outside_loop(env: Env) {
    let payload = [1u8, 2, 3];
    let _digest = env.crypto().sha256(&payload);
}

#[allow(crypto_hash_of_loop_invariant)]
fn allowed_hash(env: Env) {
    let payload = [1u8, 2, 3];
    for _ in 0..3 {
        let _digest = env.crypto().sha256(&payload);
    }
}

fn main() {}
