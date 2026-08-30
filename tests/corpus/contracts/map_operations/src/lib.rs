#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Map};

#[contract]
pub struct MapOperationsContract;

#[contractimpl]
impl MapOperationsContract {
    /// Fires `map_insert_in_loop` because Map::set/insert is called inside a loop body.
    pub fn build_balance_table_bad(env: Env, accounts: soroban_sdk::Vec<Address>) -> Map<Address, i32> {
        let mut balances: Map<Address, i32> = Map::new(&env);
        for account in accounts.iter() {
            balances.set(account, 100);
        }
        balances
    }

    /// Does not fire: accumulates totals outside or reads a map inside a loop without mutating it.
    pub fn read_balance_table_good(env: Env, accounts: soroban_sdk::Vec<Address>, table: Map<Address, i32>) -> i32 {
        let mut total = 0;
        for account in accounts.iter() {
            if let Some(bal) = table.get(account) {
                total += bal;
            }
        }
        total
    }
}
