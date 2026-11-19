#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

/// Storage key used for persistent data in this contract.
const KEY: Symbol = symbol_short!("KEY");

/// Contract demonstrating TTL (Time-To-Live) management patterns for different
/// storage types in Soroban.
///
/// This contract provides examples of how to properly extend TTL for instance,
/// persistent, and temporary storage. Proper TTL management is critical for
/// preventing data expiration and avoiding the costly archival/restore process.
#[contract]
pub struct LifecycleTtlContract;

#[contractimpl]
impl LifecycleTtlContract {
    /// Extends the TTL of the contract's instance storage.
    ///
    /// Instance storage shares the contract's TTL. This function should be
    /// called periodically (e.g., on each contract interaction) to ensure the
    /// contract instance remains live. The `threshold` parameter specifies the
    /// minimum TTL (in ledgers) below which the extension is applied, and
    /// `extend_to` specifies the new TTL to set.
    ///
    /// # Parameters
    /// - `env`: The Soroban environment.
    /// - `threshold`: Minimum current TTL (in ledgers) to trigger extension.
    /// - `extend_to`: New TTL (in ledgers) to set if threshold is met.
    pub fn bump_instance_ttl(env: Env, threshold: u32, extend_to: u32) {
        env.storage().instance().extend_ttl(threshold, extend_to);
    }

    /// Stores a value in persistent storage and sets its TTL.
    ///
    /// Persistent storage entries have independent TTLs from the contract
    /// instance. This function writes a value and immediately extends its TTL
    /// to prevent premature expiration. This pattern is essential for data
    /// that must survive longer than the default TTL.
    ///
    /// # Parameters
    /// - `env`: The Soroban environment.
    /// - `value`: The u32 value to store.
    /// - `threshold`: Minimum current TTL (in ledgers) to trigger extension.
    /// - `extend_to`: New TTL (in ledgers) to set if threshold is met.
    pub fn set_with_ttl(env: Env, value: u32, threshold: u32, extend_to: u32) {
        let storage = env.storage().persistent();
        storage.set(&KEY, &value);
        storage.extend_ttl(&KEY, threshold, extend_to);
    }

    /// Conditionally extends TTL for a temporary storage entry.
    ///
    /// Temporary storage entries are auto-evicted when their TTL expires.
    /// This function checks if the key exists before extending its TTL,
    /// avoiding a no-op extension on non-existent keys. This pattern is
    /// useful for caching scenarios where entries may have been evicted.
    ///
    /// # Parameters
    /// - `env`: The Soroban environment.
    /// - `key`: The symbol key of the temporary storage entry.
    /// - `threshold`: Minimum current TTL (in ledgers) to trigger extension.
    /// - `extend_to`: New TTL (in ledgers) to set if threshold is met.
    pub fn bump_temp_ttl(env: Env, key: Symbol, threshold: u32, extend_to: u32) {
        let storage = env.storage().temporary();
        if storage.has(&key) {
            storage.extend_ttl(&key, threshold, extend_to);
        }
    }
}