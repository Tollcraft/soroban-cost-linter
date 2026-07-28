#![no_std]
use soroban_sdk::{symbol_short, Env, Symbol};

const VALUE: Symbol = symbol_short!("value");

// Triggers soroban_storage_in_loop: while loop with storage get
pub fn bad_while(env: Env, key: i32) -> i32 {
    let mut result = 0;
    let mut i = 0;
    while i < key {
        let val: i32 = env.storage().instance().get(&VALUE).unwrap_or(0);
        result += val;
        i += 1;
    }
    result
}

// Good: while loop with storage operation outside
pub fn good_while(env: Env, count: i32) {
    let mut i = 0;
    while i < count {
        i += 1;
    }
    env.storage().instance().set(&VALUE, &i);
}