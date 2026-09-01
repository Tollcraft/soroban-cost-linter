#![no_std]
use soroban_sdk::{symbol_short, contract, contractimpl, Env, Symbol, Vec};

const SUM_KEY: Symbol = symbol_short!("sum");

// Triggers unbounded_input_loop: the loop count comes from a caller-supplied
// Vec and each iteration performs a storage write.
#[contract]
pub struct UnboundedInputLoopFixtureContract;

#[contractimpl]
impl UnboundedInputLoopFixtureContract {
    pub fn sum_and_persist(env: Env, input: Vec<u32>) -> u32 {
        let mut total = 0u32;
        for item in input.iter() {
            total = total.wrapping_add(item);
            env.storage()
                .instance()
                .set(&SUM_KEY, &total);
        }
        total
    }

    // Good: storage write happens once, outside the loop.
    pub fn sum_then_persist_once(env: Env, input: Vec<u32>) -> u32 {
        let mut total = 0u32;
        for item in input.iter() {
            total = total.wrapping_add(item);
        }
        env.storage().instance().set(&SUM_KEY, &total);
        total
    }
}
