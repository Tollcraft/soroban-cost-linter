#![no_std]
use soroban_sdk::symbol_short;
use soroban_sdk::{contract, contractimpl, vec, Address, BytesN, Env, String, Symbol, Vec};

const BALANCE: Symbol = symbol_short!("balance");
const ADMIN: Symbol = symbol_short!("admin");

#[contract]
pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    pub fn init(env: Env, admin: Address, name: String, symbol: Symbol) {
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&Symbol::new(&env, "name"), &name);
        env.storage().instance().set(&symbol, &symbol_short!("TKN"));
    }

    pub fn balance(env: Env, addr: Address) -> i128 {
        env.storage().instance().get(&addr).unwrap_or(0)
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        let mut from_bal: i128 = env.storage().instance().get(&from).unwrap_or(0);
        from_bal -= amount;
        env.storage().instance().set(&from, &from_bal);
        let mut to_bal: i128 = env.storage().instance().get(&to).unwrap_or(0);
        to_bal += amount;
        env.storage().instance().set(&to, &to_bal);
    }

    pub fn mint(env: Env, to: Address, amount: i128) {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();
        let bal: i128 = env.storage().instance().get(&to).unwrap_or(0);
        env.storage().instance().set(&to, &(bal + amount));
    }

    pub fn bulk_transfer(env: Env, from: Address, recipients: Vec<Address>, amounts: Vec<i128>) {
        from.require_auth();
        let mut from_bal: i128 = env.storage().instance().get(&from).unwrap_or(0);
        for i in 0..recipients.len() {
            let recipient = recipients.get(i).unwrap();
            let amt = amounts.get(i).unwrap();
            from_bal -= amt;
            let mut bal: i128 = env.storage().instance().get(&recipient).unwrap_or(0);
            bal += amt;
            env.storage().instance().set(&recipient, &bal);
        }
        env.storage().instance().set(&from, &from_bal);
    }

    pub fn batch_mint(env: Env, recipients: Vec<Address>, amounts: Vec<i128>) {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();
        for i in 0..recipients.len() {
            let recipient = recipients.get(i).unwrap();
            let amt = amounts.get(i).unwrap();
            let bal: i128 = env.storage().instance().get(&recipient).unwrap_or(0);
            env.storage().instance().set(&recipient, &(bal + amt));
        }
    }

    pub fn metadata(env: Env) -> Vec<Symbol> {
        let name: Symbol = env.storage().instance().get(&Symbol::new(&env, "name")).unwrap();
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        let _seq = env.ledger().sequence();
        vec![&env, name, symbol_short!("TKN")]
    }
}
