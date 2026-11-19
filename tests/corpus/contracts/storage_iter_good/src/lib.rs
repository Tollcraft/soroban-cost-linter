#![no_std]
use soroban_sdk::{symbol_short, vec, Env, Symbol, Vec};

const SAFE: Symbol = symbol_short!("safe");

/// Helper function to calculate a simple sum iteratively.
fn calculate_sum_iteratively() -> i128 {
    let mut sum = 0i128;
    for i in 0..10 {
        sum += i;
    }
    sum
}

// Good: no storage operations inside any loop construct
pub fn loop_with_buffer(env: Env) {
    let sum = calculate_sum_iteratively();
    env.storage().instance().set(&SAFE, &sum);
}

/// Helper function to calculate sum using mapped keys.
fn calculate_mapped_sum(env: &Env) -> i32 {
    let keys: Vec<i32> = vec![env, 1, 2, 3];
    keys.iter().map(|k| k * 2).sum()
}

// Good: iterator that accumulates then writes once
pub fn map_then_store(env: Env) {
    const ACCUM: Symbol = symbol_short!("accum");
    let sum = calculate_mapped_sum(&env);
    env.storage().persistent().set(&ACCUM, &sum);
}