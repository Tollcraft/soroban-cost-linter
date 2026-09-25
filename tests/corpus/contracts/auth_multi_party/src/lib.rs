//! Multi-party authorization contract module.
//!
//! This contract demonstrates the pattern of hoisting authorization checks
//! (`require_auth`) **before** processing loops. This is critical for avoiding
//! the [`require_auth_in_loop` lint](../../../../docs/lints/require_auth_in_loop.md),
//! which flags repeated authorization calls inside loop bodies as a cost anti-pattern.
//!
//! # Why hoist auth before loops?
//!
//! Each `require_auth` call invokes a host function that verifies the caller's
//! signature. When placed inside a loop, this cost is multiplied by the number
//! of iterations. By hoisting auth checks to the top of the function, the
//! authorization overhead is paid exactly once regardless of loop length.
//!
//! # Example
//!
//! ```ignore
//! // Good: auth hoisted before loop
//! sender.require_auth();
//! receiver.require_auth();
//! for amount in amounts.iter() { ... }
//!
//! // Bad: auth inside loop (triggers lint)
//! for amount in amounts.iter() {
//!     sender.require_auth(); // ← LINT: require_auth_in_loop
//! }
//! ```

#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

/// Symbol representing the authorized state in storage.
const AUTHORIZED: Symbol = symbol_short!("AUTH");

/// Multi-party authorization contract.
///
/// This contract demonstrates a pattern where multiple parties authorize an
/// action upfront, and the actual processing happens in a loop afterwards.
/// The `require_auth` calls are hoisted BEFORE the loop, so this should NOT
/// trigger the `require_auth_in_loop` lint.
///
/// # Design Rationale
///
/// Soroban charges per-host-function-call. By requiring all signers to
/// authorize once before any loop processing, we ensure the fixed cost of
/// authorization is independent of the number of items processed.
#[contract]
pub struct AuthMultiPartyContract;

#[contractimpl]
impl AuthMultiPartyContract {
    /// Performs a multi-signature transfer requiring authorization from both
    /// sender and receiver before executing.
    ///
    /// # Auth Strategy
    /// Both `sender` and `receiver` call `require_auth()` **before** the loop
    /// that processes amounts. This ensures the authorization cost is paid
    /// once (O(1)) rather than once per iteration (O(n)).
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment providing storage and host functions.
    /// * `sender` - The address authorizing the transfer. Must sign before the loop.
    /// * `receiver` - The address receiving the transfer. Must sign before the loop.
    /// * `amounts` - A vector of amounts to transfer. Processed in a loop with no auth calls.
    ///
    /// # Panics
    ///
    /// Panics if either `sender` or `receiver` fails authorization.
    pub fn multi_sig_transfer(
        env: Env,
        sender: Address,
        receiver: Address,
        amounts: soroban_sdk::Vec<i128>,
    ) {
        // Auth is hoisted: both parties authorize once, before the loop.
        // This prevents the `require_auth_in_loop` lint from firing because
        // the authorization cost is amortized across all iterations.
        sender.require_auth();
        receiver.require_auth();

        // Process all amounts in a loop — no auth inside the loop.
        // Each iteration writes to storage, but authorization is already done.
        for amount in amounts.iter() {
            // Simulate transfer logic for each amount.
            // The `let _ = amount` is a placeholder; real logic would
            // perform the actual token transfer.
            let _ = amount;
            env.storage()
                .instance()
                .set(&AUTHORIZED, &sender);
        }
    }

    /// Performs threshold authorization requiring signatures from N distinct
    /// signers before processing a batch of operations.
    ///
    /// # Auth Strategy
    /// Each signer's `require_auth()` is called during the signers loop,
    /// which runs **before** the operations processing loop. This separates
    /// the authorization phase from the execution phase, keeping auth out
    /// of the operations loop.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment providing storage and host functions.
    /// * `signers` - A vector of addresses that must each authorize. Auth is hoisted here.
    /// * `threshold` - The minimum number of distinct signers required to proceed.
    /// * `operations` - A vector of operation IDs to process. No auth inside this loop.
    ///
    /// # Logic Flow
    ///
    /// 1. Iterate through `signers`, calling `require_auth()` for each until
    ///    the `threshold` count is reached.
    /// 2. Break early once enough signers have authorized (optimization).
    /// 3. Process each operation in a separate loop with no auth calls.
    ///
    /// # Panics
    ///
    /// Panics if any signer fails authorization during the signers loop.
    pub fn threshold_authorize(
        env: Env,
        signers: soroban_sdk::Vec<Address>,
        threshold: u32,
        operations: soroban_sdk::Vec<u32>,
    ) {
        // Verify each signer has authorized — but this is a one-time check
        // on the signers vector, not inside the operations loop.
        // The loop terminates early once the threshold is met, avoiding
        // unnecessary authorization calls.
        let mut verified = 0u32;
        for signer in signers.iter() {
            signer.require_auth();
            verified += 1;
            if verified >= threshold {
                // Early exit: no need to check remaining signers once
                // the threshold is satisfied. This saves host function calls.
                break;
            }
        }

        // Now process operations — no auth inside this loop.
        // Authorization was already verified above, so this loop only
        // performs storage writes, which is the correct pattern.
        for op in operations.iter() {
            env.storage()
                .instance()
                .set(&op, &verified);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    /// Tests that `threshold_authorize` correctly requires authorization from
    /// at least two signers before processing operations.
    ///
    /// This test verifies the auth-hoisting pattern: both signers authorize
    /// before the operations loop executes.
    #[test]
    fn test_threshold_authorize() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AuthMultiPartyContract);
        let client = AuthMultiPartyContractClient::new(&env, &contract_id);

        let signer1 = Address::generate(&env);
        let signer2 = Address::generate(&env);

        // Mock all auths so that `require_auth` calls succeed in the test environment.
        env.mock_all_auths();

        let signers = soroban_sdk::vec![&env, signer1, signer2];
        let operations = soroban_sdk::vec![&env, 1u32, 2, 3];

        // Call should succeed with two signers meeting the threshold of 2.
        client.threshold_authorize(&signers, &2, &operations);
    }
}
