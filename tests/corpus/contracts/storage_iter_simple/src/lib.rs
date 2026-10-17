#![no_std]
use soroban_sdk::{symbol_short, Env, Symbol};

const COUNTER: Symbol = symbol_short!("counter");

// Triggers soroban_storage_in_loop: set inside a for loop
pub fn bad_simple(env: Env) {
    for i in 0..10 {
        env.storage().instance().set(&COUNTER, &i);
    }
}

// Good: storage operation outside the loop
pub fn good_simple(env: Env) {
    let mut total = 0i128;
    for _ in 0..10 {
        total += 1;
    }
    env.storage().instance().set(&COUNTER, &total);
}