//! Corpus contract: good persistent storage.
//!
//! This contract is a lint test fixture that demonstrates the **correct**
//! pattern for persistent storage on Soroban: one storage write per call with
//! a typed key and a typed value, with no unnecessary allocations.
//!
//! The linter uses this contract as a negative control to confirm that
//! well-formed persistent-storage usage does not trigger any warnings.

#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Symbol};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Persist a `Symbol` value under the given numeric `id` key.
    ///
    /// # Storage tier
    ///
    /// Persistent storage is used here because the data must survive ledger
    /// entry expiry without the contract re-uploading it.  Instance storage
    /// would be cheaper per write but would couple the entry lifetime to the
    /// contract instance, which is incorrect for per-item records.
    ///
    /// # Allocation behaviour
    ///
    /// `Symbol` is a value type encoded in a single 64-bit host value, so
    /// passing it by value incurs no heap allocation.  The `u32` key is
    /// similarly allocation-free.  This function therefore performs exactly
    /// one host-function call (`storage().persistent().set`) per invocation.
    pub fn add_item(env: Env, id: u32, val: Symbol) {
        env.storage().persistent().set(&id, &val);
    }
}
