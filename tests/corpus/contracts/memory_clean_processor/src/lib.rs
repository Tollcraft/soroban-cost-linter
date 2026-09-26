//! Corpus contract: memory-clean processor.
//!
//! This contract is a lint test fixture that demonstrates correct, allocation-
//! efficient byte-processing patterns on Soroban.  It is intentionally simple
//! so the linter can verify that no unnecessary host-object allocations or
//! redundant clones are introduced during in-place byte mutation.

#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Bytes, Env, Symbol};

/// Short symbol used as the event topic when publishing the processed result.
///
/// Using `symbol_short!` keeps the tag construction cost-free at runtime
/// because the macro encodes the symbol directly into a 64-bit value without
/// a host-function call.
const EVENT_TAG: Symbol = symbol_short!("event");

#[contract]
pub struct MemoryCleanProcessorContract;

#[contractimpl]
impl MemoryCleanProcessorContract {
    /// Process up to 64 bytes of input by incrementing each byte value by one
    /// (with wrapping arithmetic) and publishing the result as a contract event.
    ///
    /// # Why a fixed-size stack buffer?
    ///
    /// Soroban charges per host-object allocation.  Copying the guest bytes
    /// into a fixed `[u8; 64]` stack buffer lets us perform all mutations in
    /// Wasm linear memory before constructing a single `Bytes` host object for
    /// the return value.  This avoids allocating an intermediate mutable host
    /// object that would otherwise be discarded immediately.
    ///
    /// Input longer than 64 bytes is silently truncated; callers that need
    /// larger payloads should split them before invoking this function.
    pub fn process_buffered_data(env: Env, data: Bytes) -> Bytes {
        // Copy at most 64 bytes into a stack-allocated buffer.
        // `min(64)` ensures we never exceed the buffer length even for
        // oversized inputs, avoiding a potential panic on slice indexing.
        let mut buffer = [0u8; 64];
        let len = data.len().min(64) as usize;
        data.copy_into_slice(&mut buffer[..len]);

        // Increment each byte with wrapping addition so that a value of 0xFF
        // rolls over to 0x00 instead of panicking.  This is a deliberate
        // design choice: the contract must not trap on any valid byte value.
        for i in 0..len {
            buffer[i] = buffer[i].wrapping_add(1);
        }

        // Construct the result host object once, after all mutations are done,
        // to minimise the number of host-function calls made from Wasm.
        let result = Bytes::from_slice(&env, &buffer[..len]);

        // Publish the processed bytes as an event so off-chain indexers can
        // observe every transformation without an additional storage write.
        env.events().publish((EVENT_TAG,), result.clone());
        result
    }

    /// Compute a simple wrapping checksum over up to 32 bytes of input.
    ///
    /// Each byte is accumulated into a `u32` with `wrapping_add` so that
    /// overflow never causes a Wasm trap, regardless of input content.
    ///
    /// # Why not hash with `env.crypto()`?
    ///
    /// Cryptographic host functions (SHA-256, Keccak-256) are metered at a
    /// significantly higher cost than arithmetic inside Wasm.  For use-cases
    /// that only need a lightweight integrity tag—such as cache-busting keys—
    /// this in-Wasm accumulation is the cheaper choice.
    ///
    /// Input longer than 32 bytes is silently truncated for the same reason as
    /// `process_buffered_data`: we bound the stack buffer to a fixed size to
    /// avoid variable-cost host allocations inside this function.
    pub fn compute_hash_sum(_env: Env, input_data: Bytes) -> u32 {
        let mut hash_sum = 0u32;

        // Stack buffer bounded to 32 bytes; no heap allocation required.
        let mut buffer = [0u8; 32];
        let len = input_data.len().min(32) as usize;
        input_data.copy_into_slice(&mut buffer[..len]);

        // Accumulate bytes using wrapping arithmetic to stay trap-free.
        for byte in &buffer[..len] {
            hash_sum = hash_sum.wrapping_add(*byte as u32);
        }
        hash_sum
    }
}
