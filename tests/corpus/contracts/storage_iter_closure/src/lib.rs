#![no_std]
use soroban_sdk::{symbol_short, Env, Symbol};

const ITER_KEY: Symbol = symbol_short!("iter_key");

// Triggers soroban_storage_in_loop: iterator closure with storage set
pub fn bad_closure(env: Env) {
    let items: Vec<i32> = vec![1, 2, 3, 4, 5];
    items.into_iter().for_each(|x| {
        env.storage().instance().set(&ITER_KEY, &x);
    });
}

// Good: iterator closure without storage access
pub fn good_closure(env: Env) {
    let items: Vec<i32> = vec![1, 2, 3, 4, 5];
    let sum: i32 = items.into_iter().sum();
    env.storage().instance().set(&ITER_KEY, &sum);
}