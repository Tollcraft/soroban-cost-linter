#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

/// Storage key for the contract administrator.
const ADMIN: Symbol = symbol_short!("ADMIN");
/// Storage key for the counter value.
const COUNTER: Symbol = symbol_short!("COUNTER");

/// Basic lifecycle contract demonstrating initialization, state mutation, and
/// read patterns using instance storage.
///
/// This contract showcases common patterns for contract initialization with
/// authorization, counter incrementation, and state reads. Instance storage
/// is used here because the data is bounded (single admin, single counter)
/// and benefits from sharing the contract's TTL.
#[contract]
pub struct LifecycleBasicContract;

#[contractimpl]
impl LifecycleBasicContract {
    /// Initializes the contract with an admin address.
    ///
    /// This function is idempotent: it only sets the admin and counter if
    /// the contract hasn't been initialized yet. The admin must authorize
    /// the initialization via `require_auth()`.
    ///
    /// Using `storage.has()` here is more efficient than `storage.get()` for
    /// the initialization check because it only checks key existence without
    /// deserializing the stored value.
    ///
    /// # Parameters
    /// - `env`: The Soroban environment.
    /// - `admin`: The address to set as contract administrator.
    pub fn init(env: Env, admin: Address) {
        admin.require_auth();
        let storage = env.storage().instance();
        if !storage.has(&ADMIN) {
            storage.set(&ADMIN, &admin);
            storage.set(&COUNTER, &0u32);
        }
    }

    /// Increments the counter and returns the new value.
    ///
    /// This demonstrates a read-modify-write pattern on instance storage.
    /// The `unwrap_or(0)` handles the case where the counter hasn't been
    /// initialized (though `init` should be called first in practice).
    ///
    /// Note: For high-frequency increment scenarios, consider batching
    /// increments in memory and writing once to reduce storage operations.
    ///
    /// # Parameters
    /// - `env`: The Soroban environment.
    ///
    /// # Returns
    /// The new counter value after incrementing.
    pub fn increment(env: Env) -> u32 {
        let storage = env.storage().instance();
        let mut count: u32 = storage.get(&COUNTER).unwrap_or(0);
        count += 1;
        storage.set(&COUNTER, &count);
        count
    }

    /// Returns the current counter value.
    ///
    /// Reads from instance storage with a default of 0 if the key is not
    /// found (e.g., if `init` was not called).
    ///
    /// # Parameters
    /// - `env`: The Soroban environment.
    ///
    /// # Returns
    /// The current counter value, or 0 if not initialized.
    pub fn get_counter(env: Env) -> u32 {
        env.storage().instance().get(&COUNTER).unwrap_or(0)
    }
}