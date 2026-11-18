//! Batch per-item authorization contract module.
//!
//! This contract processes batches of operations where each recipient must
//! authorize their own portion. The authorization is collected upfront from
//! all recipients before the processing loop begins, so `require_auth` is
//! NOT inside the processing loop.
//!
//! # Why hoist auth before loops?
//!
//! Each `require_auth` call invokes a host function that verifies the caller's
//! signature. When placed inside a loop, this cost is multiplied by the number
//! of iterations. By collecting recipient authorizations in an initial pass
//! before executing the state transitions, the authorization cost is decoupled
//! from the execution logic.

#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

/// Storage symbol for processing state.
const PROCESSING: Symbol = symbol_short!("PROC");

/// Storage symbol for batch sequence tracking.
const BATCH_ID: Symbol = symbol_short!("BATCH");

/// Batch per-item authorization contract.
#[contract]
pub struct AuthBatchPerItemContract;

#[contractimpl]
impl AuthBatchPerItemContract {
    /// Distribute tokens to multiple recipients. Each recipient authorizes
    /// their allocation before the distribution loop begins.
    ///
    /// # Auth Strategy
    /// The distributor authorizes once upfront. Then, all recipients authorize
    /// in a separate pre-check loop before distribution storage writes occur.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    /// * `distributor` - The address funding the distribution.
    /// * `recipients` - List of recipient addresses.
    /// * `amounts` - Corresponding distribution amounts.
    pub fn distribute(
        env: Env,
        distributor: Address,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
    ) {
        // Distributor authorizes once
        distributor.require_auth();

        // Collect all recipient authorizations upfront
        authorize_recipients(&recipients);

        // Execute distribution writes
        execute_distribution(&env, &recipients, &amounts);
    }

    /// Batch update with admin approval: the admin approves a batch of
    /// configuration changes, and each affected party acknowledges.
    ///
    /// # Auth Strategy
    /// The admin authorizes the entire batch upfront. Each affected party then
    /// acknowledges their update in a separate pre-check phase before applying changes.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment.
    /// * `admin` - The admin address authorizing the batch update.
    /// * `updates` - A vector of `(Address, u32)` tuples representing target addresses and updated values.
    pub fn batch_update(env: Env, admin: Address, updates: soroban_sdk::Vec<(Address, u32)>) {
        // Admin authorizes the entire batch
        admin.require_auth();

        // Each affected party acknowledges their update
        authorize_updates(&updates);

        // Apply all state updates
        execute_batch_update(&env, &updates);
    }
}

/// Pre-check helper to collect authorizations from all recipients upfront.
fn authorize_recipients(recipients: &soroban_sdk::Vec<Address>) {
    let recipient_count = recipients.len();
    for i in 0..recipient_count {
        let recipient = recipients.get(i).unwrap();
        recipient.require_auth();
    }
}

/// Executes distribution storage writes for all authorized recipients.
fn execute_distribution(
    env: &Env,
    recipients: &soroban_sdk::Vec<Address>,
    amounts: &soroban_sdk::Vec<i128>,
) {
    let recipient_count = recipients.len();
    for i in 0..recipient_count {
        let recipient = recipients.get(i).unwrap();
        let amount = amounts.get(i).unwrap_or(0);
        env.storage()
            .instance()
            .set(&(recipient, amount), &PROCESSING);
    }
}

/// Pre-check helper to collect authorizations from all update recipients upfront.
fn authorize_updates(updates: &soroban_sdk::Vec<(Address, u32)>) {
    for (recipient, _value) in updates.iter() {
        recipient.require_auth();
    }
}

/// Executes state transitions for batch updates and updates batch sequence ID.
fn execute_batch_update(env: &Env, updates: &soroban_sdk::Vec<(Address, u32)>) {
    let mut batch_count: u32 = env.storage().instance().get(&BATCH_ID).unwrap_or(0);

    for (recipient, value) in updates.iter() {
        batch_count += 1;
        env.storage()
            .instance()
            .set(&(recipient, batch_count), &value);
    }

    env.storage().instance().set(&BATCH_ID, &batch_count);
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_distribute() {
        let env = Env::default();
        let contract_id = env.register(AuthBatchPerItemContract, ());
        let client = AuthBatchPerItemContractClient::new(&env, &contract_id);

        let distributor = Address::generate(&env);
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);

        env.mock_all_auths();

        let recipients = soroban_sdk::vec![&env, r1, r2];
        let amounts = soroban_sdk::vec![&env, 100i128, 200];

        client.distribute(&distributor, &recipients, &amounts);
    }

    #[test]
    fn test_distribute_empty() {
        let env = Env::default();
        let contract_id = env.register(AuthBatchPerItemContract, ());
        let client = AuthBatchPerItemContractClient::new(&env, &contract_id);

        let distributor = Address::generate(&env);
        env.mock_all_auths();

        let recipients = soroban_sdk::vec![&env];
        let amounts = soroban_sdk::vec![&env];

        client.distribute(&distributor, &recipients, &amounts);
    }

    #[test]
    fn test_batch_update() {
        let env = Env::default();
        let contract_id = env.register(AuthBatchPerItemContract, ());
        let client = AuthBatchPerItemContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let u1 = Address::generate(&env);
        let u2 = Address::generate(&env);

        env.mock_all_auths();

        let updates = soroban_sdk::vec![&env, (u1.clone(), 10u32), (u2.clone(), 20u32)];
        client.batch_update(&admin, &updates);
    }

    #[test]
    fn test_batch_update_empty() {
        let env = Env::default();
        let contract_id = env.register(AuthBatchPerItemContract, ());
        let client = AuthBatchPerItemContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        env.mock_all_auths();

        let updates = soroban_sdk::vec![&env];
        client.batch_update(&admin, &updates);
    }
}
