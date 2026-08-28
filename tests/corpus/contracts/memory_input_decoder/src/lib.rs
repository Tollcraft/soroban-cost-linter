#![no_std]
use soroban_sdk::{contract, contractimpl, Symbol, Vec};

#[contract]
pub struct MemoryInputDecoderContract;

#[contractimpl]
impl MemoryInputDecoderContract {
    pub fn verify_checksums(_env: soroban_sdk::Env, payload: Vec<u32>, expected: u32) -> bool {
        let mut sum = 0u32;
        for i in 0..payload.len() {
            if let Some(val) = payload.get(i) {
                sum = sum.wrapping_add(val);
            }
        }
        sum == expected
    }

    pub fn get_action_tag(env: soroban_sdk::Env) -> Symbol {
        Symbol::new(&env, "decode")
    }
}
