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

const DOMAIN_SEP: &[u8] = b"stellar-domain-sep";

// Fires: a byte-string literal is a compile-time constant.
fn bad_literal(env: Env) {
    let _h = env.crypto().sha256(b"fixed tag"); // Should Warn
}

// Fires: an array literal is a compile-time constant.
fn bad_array_literal(env: Env) {
    let _h = env.crypto().sha256(&[1u8, 2, 3, 4]); // Should Warn
}

// Fires: a `const` item passed directly is a compile-time constant.
fn bad_const_item(env: Env) {
    let _h = env.crypto().keccak256(DOMAIN_SEP); // Should Warn
}

// Fires: a `const` item behind an `&` is still a compile-time constant.
fn bad_const_item_ref(env: Env) {
    let _h = env.crypto().sha256(&DOMAIN_SEP); // Should Warn
}

// Good: hashing a runtime-derived function argument must NOT fire.
fn good_runtime_arg(env: Env, data: &[u8]) {
    let _h = env.crypto().sha256(data); // Good
}

// Good: a local bound to a literal is a runtime value, not a `const` item.
fn good_local_literal(env: Env) {
    let data = b"fixed tag";
    let _h = env.crypto().sha256(data); // Good
}

// Good: the input depends on a runtime parameter, so it is not constant.
fn good_runtime_array(env: Env, salt: u8) {
    let data = [1u8, 2, salt];
    let _h = env.crypto().sha256(&data); // Good
}

#[allow(crypto_hash_of_constant)]
fn allowed_const_hash(env: Env) {
    let _h = env.crypto().sha256(b"fixed tag"); // Good (allowed)
}

fn main() {}
