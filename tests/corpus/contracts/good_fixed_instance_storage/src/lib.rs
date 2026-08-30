#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Symbol};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn set_admin(env: Env, admin: Symbol) {
        env.storage().instance().set(&Symbol::new(&env, "admin"), &admin);
    }
}
