#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Vec, Symbol};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn add_item(env: Env, id: u32, val: Symbol) {
        env.storage().persistent().set(&id, &val);
    }
}
