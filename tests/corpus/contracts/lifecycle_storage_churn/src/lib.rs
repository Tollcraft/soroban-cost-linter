#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

const S_KEY: Symbol = symbol_short!("S_KEY");

#[contract]
pub struct LifecycleStorageChurnContract;

#[contractimpl]
impl LifecycleStorageChurnContract {
    pub fn update_and_clear(env: Env, new_val: u32) {
        let storage = env.storage().persistent();
        storage.set(&S_KEY, &new_val);
        if new_val == 0 {
            storage.remove(&S_KEY);
        }
    }

    pub fn churn_temporary(env: Env, key: Symbol, val: u32) {
        let storage = env.storage().temporary();
        let old: u32 = storage.get(&key).unwrap_or(0);
        let updated = old.saturating_add(val);
        storage.set(&key, &updated);
    }
}
