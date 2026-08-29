#![allow(unnecessary_host_function_call)]

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
            pub fn ed25519_verify(&self, _key: &[u8], _msg: &[u8], _sig: &[u8]) -> bool { true }
            pub fn secp256k1_recover(&self, _key: &[u8], _msg: &[u8], _sig: &[u8]) -> bool { true }
            pub fn secp256r1_verify(&self, _key: &[u8], _msg: &[u8], _sig: &[u8]) -> bool { true }
        }
    }
}

use soroban_sdk::Env;

// =======================================================================
// signature_verification_in_loop — Fixtures
// =======================================================================

// --- Positive (should warn): signature verification inside a loop ---

fn bad_ed25519_verify_in_for_loop(env: Env) {
    let key = [0u8; 32];
    let msg = [0u8; 32];
    let sig = [0u8; 64];
    for _ in 0..10 {
        env.crypto().ed25519_verify(&key, &msg, &sig); // Should Warn
    }
}

fn bad_ed25519_verify_in_while_loop(env: Env) {
    let key = [0u8; 32];
    let msg = [0u8; 32];
    let sig = [0u8; 64];
    let mut i = 0;
    while i < 10 {
        env.crypto().ed25519_verify(&key, &msg, &sig); // Should Warn
        i += 1;
    }
}

fn bad_ed25519_verify_in_loop_loop(env: Env) {
    let key = [0u8; 32];
    let msg = [0u8; 32];
    let sig = [0u8; 64];
    let mut count = 0;
    loop {
        env.crypto().ed25519_verify(&key, &msg, &sig); // Should Warn
        count += 1;
        if count >= 5 {
            break;
        }
    }
}

fn bad_secp256r1_verify_in_loop(env: Env) {
    let key = [0u8; 32];
    let msg = [0u8; 32];
    let sig = [0u8; 64];
    for _ in 0..10 {
        env.crypto().secp256r1_verify(&key, &msg, &sig); // Should Warn
    }
}

// --- Negative (should not warn): verification outside a loop ---

fn good_signature_verification_outside_loop(env: Env) {
    let key = [0u8; 32];
    let msg = [0u8; 32];
    let sig = [0u8; 64];
    env.crypto().ed25519_verify(&key, &msg, &sig); // Good — called once
}

// --- Suppression test ---

#[allow(signature_verification_in_loop)]
fn allowed_signature_verification_in_loop(env: Env) {
    let key = [0u8; 32];
    let msg = [0u8; 32];
    let sig = [0u8; 64];
    for _ in 0..10 {
        env.crypto().ed25519_verify(&key, &msg, &sig); // Good (allowed)
    }
}

fn main() {}
