#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

const KEY: Symbol = symbol_short!("KEY");

#[contract]
pub struct LifecycleTtlContract;

#[contractimpl]
impl LifecycleTtlContract {
    pub fn bump_instance_ttl(env: Env, threshold: u32, extend_to: u32) {
        env.storage().instance().extend_ttl(threshold, extend_to);
    }

    pub fn set_with_ttl(env: Env, value: u32, threshold: u32, extend_to: u32) {
        let storage = env.storage().persistent();
        storage.set(&KEY, &value);
        storage.extend_ttl(&KEY, threshold, extend_to);
    }

    pub fn bump_temp_ttl(env: Env, key: Symbol, threshold: u32, extend_to: u32) {
        let storage = env.storage().temporary();
        if storage.has(&key) {
            storage.extend_ttl(&key, threshold, extend_to);
        }
    }
}
