#![no_std]
use soroban_sdk::{contract, contractimpl, BytesN, Env};

#[contract]
pub struct CryptoInLoopVolatileContract;

#[contractimpl]
impl CryptoInLoopVolatileContract {
    pub fn verify_outside(env: Env, pk: BytesN<32>, msg: BytesN<32>, sig: BytesN<64>) {
        env.crypto().ed25519_verify(&pk, &msg, &sig);
    }
}
