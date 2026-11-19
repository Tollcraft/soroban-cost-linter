#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol, Vec};

const SUM_KEY: Symbol = symbol_short!("sum");

// Triggers unbounded_input_loop: the loop count comes from a caller-supplied
// Vec and each iteration performs a storage write.
#[contract]
pub struct UnboundedInputLoopFixtureContract;

#[contractimpl]
impl UnboundedInputLoopFixtureContract {
    pub fn sum_and_persist(env: Env, input: Vec<u32>) -> u32 {
        let mut total = 0u32;
        // Cache storage instance to avoid repeated getter calls
        let storage = env.storage().instance();
        for item in input.iter() {
            total = total.wrapping_add(item);
            storage.set(&SUM_KEY, &total);
        }
        total
    }

    // Good: storage write happens once, outside the loop.
    pub fn sum_then_persist_once(env: Env, input: Vec<u32>) -> u32 {
        let mut total = 0u32;
        // Use iterator without unnecessary intermediate allocations
        for item in input.iter() {
            total = total.wrapping_add(item);
        }
        // Cache storage instance reference to avoid redundant calls
        let storage = env.storage().instance();
        storage.set(&SUM_KEY, &total);
        total
    }
}
