#![no_std]

use soroban_sdk::{symbol_short, contract, contractimpl, Env, Symbol};

const COUNTER: Symbol = symbol_short!("counter");

/// A contract demonstrating bounded input loops where the iteration
/// count is known at compile time.
///
/// These loops are safe because the bound is a constant, making
/// resource consumption predictable and input-independent.
#[contract]
pub struct BoundedInputLoopFixtureContract;

#[contractimpl]
impl BoundedInputLoopFixtureContract {
    /// Computes the sum of integers from 0 to 9 using a bounded loop.
    ///
    /// The loop iterates exactly 10 times — a compile-time constant —
    /// ensuring predictable resource consumption. Each iteration
    /// updates the storage counter with the running total.
    ///
    /// # Returns
    /// The final accumulated sum after all iterations.
    pub fn bounded_sum(env: Env) -> u32 {
        let mut total = 0u32;
        // Bounded by the constant `10` — iteration count is fixed.
        for i in 0..10 {
            total = total.wrapping_add(i);
            env.storage().instance().set(&COUNTER, &total);
        }
        total
    }

    /// Processes a bounded range of inputs and returns the count
    /// of iterations performed.
    ///
    /// This function demonstrates that storage operations inside
    /// a loop with a constant bound are acceptable since the
    /// resource cost is deterministic and input-independent.
    pub fn process_bounded(env: Env) -> u32 {
        let mut count = 0u32;
        for i in 0..10 {
            env.storage().instance().set(&COUNTER, &(i + count));
            count += 1;
        }
        count
    }

    /// Performs a fixed number of storage reads and writes.
    ///
    /// The loop bound of 10 is a compile-time constant, making
    /// this a safe pattern for bounded input scenarios.
    pub fn bounded_read_write(env: Env) -> u32 {
        let mut result = 0u32;
        for i in 0..10 {
            result = env.storage().instance().get(&COUNTER).unwrap_or(0);
            env.storage().instance().set(&COUNTER, &(result + i));
        }
        result
    }
}
