#![no_std]
use soroban_sdk::{symbol_short, vec, Env, Symbol, Vec};

const SAFE: Symbol = symbol_short!("safe");

// Good: no storage operations inside any loop construct
pub fn loop_with_buffer(env: Env) {
    let mut sum = 0i128;
    for i in 0..10 {
        sum += i;
    }
    env.storage().instance().set(&SAFE, &sum);
}

// Good: iterator that accumulates then writes once
pub fn map_then_store(env: Env) {
    const ACCUM: Symbol = symbol_short!("accum");
    let keys: Vec<i32> = vec![&env, 1, 2, 3];
    let sum: i32 = keys.iter().map(|k| k * 2).sum();
    env.storage().persistent().set(&ACCUM, &sum);
}