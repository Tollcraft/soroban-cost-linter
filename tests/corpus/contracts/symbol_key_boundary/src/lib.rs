#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

#[contract]
pub struct SymbolKeyBoundaryContract;

#[contractimpl]
impl SymbolKeyBoundaryContract {
    pub fn test_symbols(env: Env) {
        let _s1 = Symbol::new(&env, "short_9");
        let _s2 = Symbol::new(&env, "a");
        let _l1 = Symbol::new(&env, "longer_than_nine");
        let _m1 = symbol_short!("short_9");
    }
}
