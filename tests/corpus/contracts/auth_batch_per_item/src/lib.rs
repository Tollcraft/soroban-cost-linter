#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

const PROCESSING: Symbol = symbol_short!("PROC");
const BATCH_ID: Symbol = symbol_short!("BATCH");

/// Batch per-item authorization contract.
///
/// This contract processes batches of operations where each recipient must
/// authorize their own portion. The authorization is collected upfront from
/// all recipients before the processing loop begins, so `require_auth` is
/// NOT inside the processing loop.
#[contract]
pub struct AuthBatchPerItemContract;

#[contractimpl]
impl AuthBatchPerItemContract {
    /// Distribute tokens to multiple recipients. Each recipient authorizes
    /// their allocation before the distribution loop begins.
    ///
    /// This pattern is correct: auth is hoisted. The lint should NOT fire.
    pub fn distribute(
        env: Env,
        distributor: Address,
        recipients: soroban_sdk::Vec<Address>,
        amounts: soroban_sdk::Vec<i128>,
    ) {
        // Distributor authorizes once
        distributor.require_auth();

        // Collect all recipient authorizations before processing
        // (this is a pre-check loop, not the processing loop)
        let recipient_count = recipients.len();
        for i in 0..recipient_count {
            let recipient = recipients.get(i).unwrap();
            recipient.require_auth();
        }

        // Now distribute — no auth inside this loop
        for i in 0..recipient_count {
            let recipient = recipients.get(i).unwrap();
            let amount = amounts.get(i).unwrap_or(0);
            env.storage()
                .instance()
                .set(&(recipient, amount), &PROCESSING);
        }
    }

    /// Batch update with admin approval: the admin approves a batch of
    /// configuration changes, and each affected party acknowledges.
    pub fn batch_update(
        env: Env,
        admin: Address,
        updates: soroban_sdk::Vec<(Address, u32)>,
    ) {
        // Admin authorizes the entire batch
        admin.require_auth();

        // Each affected party acknowledges their update
        // (auth is collected in a separate pass before applying)
        for (recipient, _value) in updates.iter() {
            recipient.require_auth();
        }

        // Apply all updates — no auth inside this loop
        let mut batch_count: u32 = env
            .storage()
            .instance()
            .get(&BATCH_ID)
            .unwrap_or(0);
        for (recipient, value) in updates.iter() {
            batch_count += 1;
            env.storage()
                .instance()
                .set(&(recipient, batch_count), &value);
        }
        env.storage()
            .instance()
            .set(&BATCH_ID, &batch_count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_distribute() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AuthBatchPerItemContract);
        let client = AuthBatchPerItemContractClient::new(&env, &contract_id);

        let distributor = Address::generate(&env);
        let r1 = Address::generate(&env);
        let r2 = Address::generate(&env);

        env.mock_all_auths();

        let recipients = soroban_sdk::vec![&env, r1, r2];
        let amounts = soroban_sdk::vec![&env, 100i128, 200];

        client.distribute(&distributor, &recipients, &amounts);
    }
}
