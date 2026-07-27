#![no_std]
use soroban_sdk::{symbol_short, Env, Symbol};

const MIXED: Symbol = symbol_short!("mixed");

// Triggers soroban_storage_in_loop: storage get inside a loop
pub fn bad_mixed_get(env: Env) {
    for _ in 0..10 {
        let _val: i32 = env.storage().instance().get(&MIXED).unwrap_or(0);
    }
}

// Triggers soroban_storage_in_loop: storage set inside a loop with different storage type
pub fn bad_mixed_persistent(env: Env) {
    for i in 0..10 {
        env.storage().persistent().set(&MIXED, &i);
    }
}

// Good: loop accumulates in memory, single storage write after
pub fn good_mixed(env: Env) {
    let mut total = 0i128;
    for i in 0..10 {
        total += i;
    }
    env.storage().instance().set(&MIXED, &total);
}