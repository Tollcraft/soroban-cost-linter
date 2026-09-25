#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Symbol};

#[contract]
pub struct SymbolKeyEventTopicsContract;

#[contractimpl]
impl SymbolKeyEventTopicsContract {
    pub fn emit_events_in_loop(env: Env, count: u32) {
        let topic1 = Symbol::new(&env, "topic_one");
        let topic2 = Symbol::new(&env, "topic_two");
        for i in 0..count {
            env.events().publish((topic1.clone(), topic2.clone()), i);
        }
    }
}
