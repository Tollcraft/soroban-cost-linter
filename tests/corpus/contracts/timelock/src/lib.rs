#![no_std]
use soroban_sdk::symbol_short;
use soroban_sdk::{contract, contractimpl, vec, Address, Env, Map, Symbol, Vec};

const LOCKED: Symbol = symbol_short!("locked");
const TOTAL_LOCKED: Symbol = symbol_short!("tot_lock");

#[contract]
pub struct TimelockContract;

#[contractimpl]
impl TimelockContract {
    pub fn init(env: Env, admin: Address) {
        env.storage().instance().set(&symbol_short!("admin"), &admin);
    }

    pub fn lock(env: Env, user: Address, amount: i128, unlock_time: u64) {
        let key = (LOCKED, user.clone());
        let existing: Map<u64, i128> = env.storage().instance().get(&key).unwrap_or(Map::new(&env));
        let mut entries = existing;
        entries.set(unlock_time, amount);
        env.storage().instance().set(&key, &entries);

        let total: i128 = env.storage().instance().get(&TOTAL_LOCKED).unwrap_or(0);
        env.storage().instance().set(&TOTAL_LOCKED, &(total + amount));
    }

    pub fn release(env: Env, user: Address, at_time: u64) {
        let key = (LOCKED, user.clone());
        let mut entries: Map<u64, i128> = env.storage().instance().get(&key).unwrap_or(Map::new(&env));
        let amount = entries.get(at_time).unwrap_or(0);
        if amount > 0 {
            entries.remove(at_time);
            env.storage().instance().set(&key, &entries);
            let _seq = env.ledger().sequence();
        }
    }

    pub fn release_all(env: Env, user: Address, before_time: u64) {
        let key = (LOCKED, user.clone());
        let entries: Map<u64, i128> = env.storage().instance().get(&key).unwrap_or(Map::new(&env));
        let mut remaining = Map::new(&env);
        for (t, amt) in entries.iter() {
            if t >= before_time {
                remaining.set(t, amt);
            } else {
                let balance: i128 = env.storage().instance().get(&user).unwrap_or(0);
                env.storage().instance().set(&user, &(balance + amt));
            }
        }
        env.storage().instance().set(&key, &remaining);
    }

    pub fn summary(env: Env, user: Address) -> Vec<Symbol> {
        let key = (LOCKED, user.clone());
        let entries: Map<u64, i128> = env.storage().instance().get(&key).unwrap_or(Map::new(&env));
        let mut result = vec![&env];
        for (t, amt) in entries.iter() {
            let _seq = env.ledger().sequence();
            let _sym = Symbol::new(&env, "entry");
            result.push_back(_sym);
        }
        result
    }
}
