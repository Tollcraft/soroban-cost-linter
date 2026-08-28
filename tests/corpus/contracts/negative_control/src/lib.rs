#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Map, Symbol, Vec};

const KEY: Symbol = symbol_short!("KEY");
const STATE: Symbol = symbol_short!("STATE");

#[contract]
pub struct NegativeControlContract;

#[contractimpl]
impl NegativeControlContract {
    pub fn process_data(env: soroban_sdk::Env, count: u32) -> u32 {
        let storage = env.storage().instance();
        let current_state: u32 = storage.get(&STATE).unwrap_or(0);

        let mut total = current_state;
        let limit = count.min(100);
        for i in 0..limit {
            total = total.saturating_add(i);
        }

        storage.set(&STATE, &total);

        let mut map: Map<u32, u32> = Map::new(&env);
        map.set(1, total);
        map.set(2, total.saturating_add(1));

        let vec: Vec<u32> = soroban_sdk::vec![&env, total, total + 1];
        let _len = vec.len();

        total
    }

    pub fn lookup_key(env: soroban_sdk::Env) -> bool {
        let storage = env.storage().persistent();
        let exists = storage.has(&KEY);
        if !exists {
            storage.set(&KEY, &1u32);
        }
        exists
    }
}
