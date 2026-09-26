//! Corpus contract: symbol-key enum storage.
//!
//! This contract is a lint test fixture for the `soroban_storage_in_loop`
//! lint.  It deliberately triggers the lint by writing to instance storage
//! inside a loop, using an enum variant as the storage key.
//!
//! The enum-keyed pattern is tested separately from the raw-`Symbol` keyed
//! pattern to ensure the lint detects anti-patterns regardless of the key
//! type used.

#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Env, Symbol};

/// Storage key enum for this contract.
///
/// Using a `#[contracttype]` enum as a storage key is the idiomatic Soroban
/// pattern because it encodes variant information into the host's storage
/// layer, preventing accidental key collisions between different data types.
#[contracttype]
pub enum DataKey {
    /// Per-user storage slot, disambiguated by a `Symbol` user identifier.
    User(Symbol),
    /// Singleton admin slot.
    Admin,
}

#[contract]
pub struct SymbolKeyEnumStorageContract;

#[contractimpl]
impl SymbolKeyEnumStorageContract {
    /// Write a counter value to instance storage for each iteration, keyed by
    /// a freshly constructed `DataKey::User` variant.
    ///
    /// # Why this triggers `soroban_storage_in_loop`
    ///
    /// Each call to `env.storage().instance().set` crosses the Wasm–host
    /// boundary and is metered by the Soroban fee schedule.  Performing `count`
    /// such writes in a loop means the total cost scales linearly with `count`,
    /// which is an input-dependent, unbounded cost growth — exactly the class
    /// of anti-pattern this linter is designed to detect.
    ///
    /// The idiomatic fix is to accumulate mutations into a guest-side
    /// collection (e.g. a `Map`) and flush them to storage once, outside the
    /// loop.  See `good_persistent_storage` for a single-write reference.
    pub fn construct_keys_in_loop(env: Env, count: u32) {
        for i in 0..count {
            // `Symbol::new` is a host-function call whose cost is proportional
            // to the string length.  Constructing it inside the loop compounds
            // the per-iteration cost beyond the storage write alone.
            let key = DataKey::User(Symbol::new(&env, "user_key"));
            env.storage().instance().set(&key, &i);
        }
    }
}
