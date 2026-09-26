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
    /// This function minimizes `Symbol` clones by only cloning once per key
    /// for `contains_key` (which takes ownership), then reusing the original
    /// key for `get`. This reduces per-iteration allocation overhead compared
    /// to cloning for both calls.
    pub fn find_in_collection(_env: Env, keys: Vec<Symbol>, map: Map<Symbol, i32>) -> i32 {
        let mut sum = 0i32;
        for key in keys.iter() {
            // Check if the key exists in the map before attempting to retrieve.
            // `contains_key` takes ownership, so we clone the key here.
            // The original key is then reused for `get`, avoiding a second clone.
            if map.contains_key(key.clone()) {
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
