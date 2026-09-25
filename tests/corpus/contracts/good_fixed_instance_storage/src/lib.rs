#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Symbol};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn set_admin(env: Env, admin: Symbol) {
        write_admin(&env, &admin);
    }
}

/// Helper function to encapsulate storage access logic
pub fn write_admin(env: &Env, admin: &Symbol) {
    let admin_key = Symbol::new(env, "admin");
    env.storage().instance().set(&admin_key, admin);
}
