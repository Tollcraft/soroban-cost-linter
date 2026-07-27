#![no_std]
use soroban_sdk::{symbol_short, Env, Symbol};

const DATA: Symbol = symbol_short!("data");

// Triggers soroban_storage_in_loop: nested loops with storage set
pub fn bad_nested(env: Env) {
    for i in 0..5 {
        for j in 0..5 {
            env.storage().instance().set(&DATA, &(i + j));
        }
    }
}

// Good: nested loop with storage operation outside
pub fn good_nested(env: Env) {
    let mut sum = 0i128;
    for i in 0..5 {
        for j in 0..5 {
            sum += i + j;
        }
    }
    env.storage().instance().set(&DATA, &sum);
}