//! Compute collection search test fixture contract module.
//!
//! This contract provides a test case for collection search patterns,
//! demonstrating efficient iteration over a collection of keys against a Map.

#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Map, Symbol, Vec};

/// Contract for testing efficient collection lookup patterns.
///
/// Iterates over a collection of keys and sums the values associated with
/// those keys in a given Map.
#[contract]
pub struct ComputeCollectionSearchContract;

#[contractimpl]
impl ComputeCollectionSearchContract {
    /// Sums values from a Map for all keys present in the given collection.
    ///
    /// # Arguments
    ///
    /// * `_env` - The Soroban environment (unused; reserved for future extensions).
    /// * `keys` - A vector of `Symbol` keys to look up in the map.
    /// * `map` - A `Map<Symbol, i32>` containing key-value pairs to search.
    ///
    /// # Returns
    ///
    /// The sum of all values found for the keys that exist in the map.
    /// Keys not present in the map are silently skipped.
    ///
    /// # Performance
    ///
    /// This function avoids unnecessary `Symbol` clones by passing references
    /// to `contains_key` and `get`, reducing per-iteration allocation overhead.
    /// Each key is looked up exactly once, and the result is accumulated in-place.
    pub fn find_in_collection(_env: Env, keys: Vec<Symbol>, map: Map<Symbol, i32>) -> i32 {
        let mut sum = 0i32;
        for key in keys.iter() {
            // Check if the key exists in the map before attempting to retrieve.
            // Passing `key` by reference avoids cloning the Symbol, which saves
            // a memory allocation per iteration.
            if map.contains_key(key) {
                // Safe unwrap: we just confirmed the key exists above.
                // Using `if let` instead of `unwrap()` avoids panicking on
                // edge cases and makes the control flow explicit.
                if let Some(val) = map.get(key) {
                    sum += val;
                }
            }
        }
        sum
    }
}
