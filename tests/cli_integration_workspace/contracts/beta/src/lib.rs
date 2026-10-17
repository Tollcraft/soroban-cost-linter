#![no_std]
use soroban_sdk::{symbol_short, Env, Symbol};

const SAFE: Symbol = symbol_short!("safe");

pub fn good_no_loop(env: Env) {
    env.storage().instance().set(&SAFE, &42i32);
}
