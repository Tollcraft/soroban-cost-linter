#![no_std]

use soroban_sdk::symbol_short;
use soroban_sdk::{vec, Env, Symbol, Vec};

const ITER_KEY: Symbol = symbol_short!("iter_key");

/// Iterates over items and stores each value to storage.
///
/// This pattern triggers the [`soroban_storage_in_loop`] lint because
/// storage writes occur inside a closure-based iteration.
pub fn bad_closure(env: Env) {
    let items: Vec<i32> = vec![&env, 1, 2, 3, 4, 5];
    items.into_iter().for_each(|x| {
        env.storage().instance().set(&ITER_KEY, &x);
    });
}

/// Iterates over items, computes a sum in memory,
/// then performs a single storage write.
///
/// This pattern is efficient because it aggregates values
/// in memory and writes to storage only once.
pub fn good_closure(env: Env) {
    let items: Vec<i32> = vec![&env, 1, 2, 3, 4, 5];
    let sum: i32 = items.into_iter().sum();
    env.storage().instance().set(&ITER_KEY, &sum);
}

/// Doubles each item in the Vec and returns the transformed results.
///
/// This separates the transformation logic from the storage logic
/// for better testability and modularity.
fn double_items(items: Vec<i32>, env: &Env) -> Vec<i32> {
    let mut result = Vec::new(env);
    items.into_iter().for_each(|x| {
        result.push_back(x * 2);
    });
    result
}

/// Computes the sum of a Vec of integers.
///
/// This is a simple aggregation helper that avoids storage
/// operations entirely.
fn compute_sum(items: Vec<i32>) -> i32 {
    items.into_iter().sum()
}

/// Stores a single aggregated value to storage after
/// processing all items in memory.
fn store_aggregate(env: &Env, value: i32) {
    env.storage().instance().set(&ITER_KEY, &value);
}

/// Processes a batch of items by transforming them in memory
/// and storing the result. This separates the iteration logic
/// from the storage logic for better testability.
pub fn process_items(env: Env, items: Vec<i32>) {
    let doubled = double_items(items, &env);
    let total = compute_sum(doubled);
    store_aggregate(&env, total);
}
