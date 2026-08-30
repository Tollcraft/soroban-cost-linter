#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Vec, Symbol};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn push(env: Env, val: Symbol) {
        let mut vec: Vec<Symbol> = env.storage().instance().get(&Symbol::new(&env, "data")).unwrap_or(Vec::new(&env));
        vec.push_back(val);
        env.storage().instance().set(&Symbol::new(&env, "data"), &vec);
    }
}
