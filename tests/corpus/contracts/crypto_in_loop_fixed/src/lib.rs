#![no_std]
use soroban_sdk::{contract, contractimpl, Bytes, BytesN, Env};

#[contract]
pub struct CryptoInLoopFixed;

#[contractimpl]
impl CryptoInLoopFixed {
    pub fn verify_loop(env: Env, pk: BytesN<32>, msg: Bytes, sig: BytesN<64>) {
        for _ in 0..5 {
            env.crypto().ed25519_verify(&pk, &msg, &sig);
        }
    }
}
