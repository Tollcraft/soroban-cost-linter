#![no_std]

//! This module contains a test corpus contract for the Soroban cost linter.
//! It demonstrates a secure and cost-efficient pattern where cryptographic
//! operations are correctly placed outside of iterative loops. This ensures
//! that resource consumption remains bounded and predictable.

use soroban_sdk::{contract, contractimpl, Bytes, BytesN, Env};

/// A test contract illustrating safe cryptographic verification.
#[contract]
pub struct CryptoInLoopVolatileContract;

#[contractimpl]
impl CryptoInLoopVolatileContract {
    /// Verifies an Ed25519 signature.
    /// 
    /// This function performs a single, bounded cryptographic verification operation.
    /// It avoids placing `ed25519_verify` in a loop, ensuring that the CPU and memory 
    /// cost of execution remains fixed, in compliance with Soroban's resource limits.
    ///
    /// # Arguments
    ///
    /// * `env` - The current Soroban execution environment.
    /// * `pk` - A 32-byte Ed25519 public key.
    /// * `msg` - The message data that was signed.
    /// * `sig` - A 64-byte Ed25519 signature.
    pub fn verify_outside(env: Env, pk: BytesN<32>, msg: Bytes, sig: BytesN<64>) {
        env.crypto().ed25519_verify(&pk, &msg, &sig);
    }
}
