#![no_std]
use soroban_sdk::{symbol_short, Env, Symbol};

const COUNTER: Symbol = symbol_short!("counter");

pub fn bad_storage_in_loop(env: Env) {
    for _ in 0..10 {
        env.storage().instance().set(&COUNTER, &1i32);
    }
}
