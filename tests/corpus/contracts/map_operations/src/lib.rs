//! Map operations test fixture contract module.
//!
//! This contract provides test cases for the [`map_insert_in_loop` lint](../../../../docs/lints/map_insert_in_loop.md).
//! It demonstrates both the anti-pattern (Map insertions inside a loop) and the correct pattern
//! (reading from a Map inside a loop without mutating it).
//!
//! # Anti-pattern Example
//!
//! Calling `Map::set` inside a loop body causes repeated storage writes per iteration,
//! which is flagged by the `map_insert_in_loop` lint. Each storage write has a fixed
//! cost, and performing it N times in a loop results in O(N) storage operations.
//!
//! # Correct Pattern Example
//!
//! Reading from a `Map` inside a loop does not mutate state and is therefore acceptable.
//! The storage is only read, not written, so the cost is O(1) per read rather than O(N) writes.

#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Map};

/// Contract for testing Map operation patterns and their cost implications.
#[contract]
pub struct MapOperationsContract;

#[contractimpl]
impl MapOperationsContract {
    /// Builds a balance table by inserting an initial balance for each account.
    ///
    /// # Lint Warning
    ///
    /// This function intentionally triggers the `map_insert_in_loop` lint because
    /// `Map::set` is called inside a loop body. Each iteration performs a storage
    /// write, resulting in O(N) storage operations where N is the number of accounts.
    ///
    /// # Cost Analysis
    ///
    /// - Each `balances.set(account, 100)` invocation writes to instance storage.
    /// - Storage writes are among the most expensive Soroban operations.
    /// - With N accounts, this function costs N times the base storage write cost.
    ///
    /// # Optimization
    ///
    /// To avoid this, accumulate changes in memory (e.g., a `Vec` or local `Map`)
    /// and apply them in a single batch operation after the loop.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment providing storage access.
    /// * `accounts` - A vector of account addresses to create balance entries for.
    ///
    /// # Returns
    ///
    /// A `Map<Address, i32>` mapping each account to an initial balance of 100.
    pub fn build_balance_table_bad(env: Env, accounts: soroban_sdk::Vec<Address>) -> Map<Address, i32> {
        // Allocate a new empty Map for storing balances.
        // Each insertion inside the loop triggers a storage write operation.
        let mut balances: Map<Address, i32> = Map::new(&env);
        for account in accounts.iter() {
            // Anti-pattern: storing to Map inside a loop.
            // This triggers `map_insert_in_loop` because each `set` is a
            // storage write that must be metered and persisted on-chain.
            balances.set(account, 100);
        }
        balances
    }

    /// Reads balances from an existing Map and accumulates the total.
    ///
    /// # Lint Status
    ///
    /// This function does **not** trigger the `map_insert_in_loop` lint because
    /// `Map::get` only reads from storage — it does not mutate the Map.
    /// Reads inside a loop are acceptable when no state is being modified.
    ///
    /// # Cost Analysis
    ///
    /// - Each `table.get(account)` is a storage read operation.
    /// - Reads are cheaper than writes but still have a non-trivial cost.
    /// - Since only reads are performed, the cost is O(N) reads instead of O(N) writes.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment (unused in this function but required for consistency).
    /// * `accounts` - A vector of account addresses to look up in the table.
    /// * `table` - An existing `Map<Address, i32>` containing account balances.
    ///
    /// # Returns
    ///
    /// The sum of all balances found in the table for the given accounts.
    /// Accounts not present in the table contribute 0 to the total.
    pub fn read_balance_table_good(env: Env, accounts: soroban_sdk::Vec<Address>, table: Map<Address, i32>) -> i32 {
        // Accumulate the total balance without mutating the Map.
        // This loop only reads from the table, so it does not trigger any lint.
        let mut total = 0;
        for account in accounts.iter() {
            // `table.get(account)` performs a read-only lookup.
            // If the account is not found, `None` is returned and the balance is skipped.
            if let Some(bal) = table.get(account) {
                total += bal;
            }
        }
        total
    }
}
