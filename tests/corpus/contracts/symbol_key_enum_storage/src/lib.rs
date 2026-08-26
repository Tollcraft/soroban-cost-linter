#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Env, Symbol};

#[contracttype]
pub enum DataKey {
    User(Symbol),
    Admin,
}

#[contract]
pub struct SymbolKeyEnumStorageContract;

#[contractimpl]
impl SymbolKeyEnumStorageContract {
    pub fn construct_keys_in_loop(env: Env, count: u32) {
        for i in 0..count {
            let key = DataKey::User(Symbol::new(&env, "user_key"));
            env.storage().instance().set(&key, &i);
        }
    }
}
