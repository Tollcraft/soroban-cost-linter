#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Bytes, Env, Symbol};

const EVENT_TAG: Symbol = symbol_short!("event");

#[contract]
pub struct MemoryCleanProcessorContract;

#[contractimpl]
impl MemoryCleanProcessorContract {
    pub fn process_buffered_data(env: Env, data: Bytes) -> Bytes {
        let mut buffer = [0u8; 64];
        let len = data.len().min(64) as usize;
        data.copy_into_slice(&mut buffer[..len]);

        for i in 0..len {
            buffer[i] = buffer[i].wrapping_add(1);
        }

        let result = Bytes::from_slice(&env, &buffer[..len]);
        env.events().publish((EVENT_TAG,), result.clone());
        result
    }

    pub fn compute_hash_sum(_env: Env, input_data: Bytes) -> u32 {
        let mut hash_sum = 0u32;
        let mut buffer = [0u8; 32];
        let len = input_data.len().min(32) as usize;
        input_data.copy_into_slice(&mut buffer[..len]);

        for byte in &buffer[..len] {
            hash_sum = hash_sum.wrapping_add(*byte as u32);
        }
        hash_sum
    }
}
