#![no_std]
use soroban_sdk::{symbol_short, contract, contractimpl, Env, Symbol};

const COUNTER: Symbol = symbol_short!("counter");

// Negative control: the loop count is a compile-time constant, so the number
// of iterations cannot be controlled by a caller. Storage writes happen inside
// the loop, but this must NOT trigger unbounded_input_loop.
#[contract]
pub struct BoundedInputLoopFixtureContract;

#[contractimpl]
impl BoundedInputLoopFixtureContract {
    pub fn bounded_sum(env: Env) -> u32 {
        let mut total = 0u32;
        for i in 0..10 {
            total = total.wrapping_add(i);
            env.storage().instance().set(&COUNTER, &total);
        }
        total
    }
}
