#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

const ADMIN: Symbol = symbol_short!("ADMIN");
const COUNTER: Symbol = symbol_short!("COUNTER");

#[contract]
pub struct LifecycleBasicContract;

#[contractimpl]
impl LifecycleBasicContract {
    pub fn init(env: Env, admin: Address) {
        admin.require_auth();
        let storage = env.storage().instance();
        if !storage.has(&ADMIN) {
            storage.set(&ADMIN, &admin);
            storage.set(&COUNTER, &0u32);
        }
    }

    pub fn increment(env: Env) -> u32 {
        let storage = env.storage().instance();
        let mut count: u32 = storage.get(&COUNTER).unwrap_or(0);
        count += 1;
        storage.set(&COUNTER, &count);
        count
    }

    pub fn get_counter(env: Env) -> u32 {
        env.storage().instance().get(&COUNTER).unwrap_or(0)
    }
}
