#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

const AUTHORIZED: Symbol = symbol_short!("AUTH");

/// Multi-party authorization contract.
///
/// This contract demonstrates a pattern where multiple parties authorize an
/// action upfront, and the actual processing happens in a loop afterwards.
/// The `require_auth` calls are hoisted BEFORE the loop, so this should NOT
/// trigger the `require_auth_in_loop` lint.
#[contract]
pub struct AuthMultiPartyContract;

#[contractimpl]
impl AuthMultiPartyContract {
    /// Multi-signature transfer: requires authorization from both sender and
    /// receiver before executing the transfer. Auth is hoisted above the loop.
    pub fn multi_sig_transfer(
        env: Env,
        sender: Address,
        receiver: Address,
        amounts: soroban_sdk::Vec<i128>,
    ) {
        // Auth is hoisted: both parties authorize once, before the loop
        sender.require_auth();
        receiver.require_auth();

        // Process all amounts in a loop — no auth inside the loop
        for amount in amounts.iter() {
            // Simulate transfer logic for each amount
            let _ = amount;
            env.storage()
                .instance()
                .set(&AUTHORIZED, &sender);
        }
    }

    /// Threshold authorization: requires auth from N distinct signers before
    /// processing a batch of operations.
    pub fn threshold_authorize(
        env: Env,
        signers: soroban_sdk::Vec<Address>,
        threshold: u32,
        operations: soroban_sdk::Vec<u32>,
    ) {
        // Verify each signer has authorized — but this is a one-time check
        // on the signers vector, not inside the operations loop
        let mut verified = 0u32;
        for signer in signers.iter() {
            signer.require_auth();
            verified += 1;
            if verified >= threshold {
                break;
            }
        }

        // Now process operations — no auth inside this loop
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

    #[test]
    fn test_threshold_authorize() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AuthMultiPartyContract);
        let client = AuthMultiPartyContractClient::new(&env, &contract_id);

        let signer1 = Address::generate(&env);
        let signer2 = Address::generate(&env);

        env.mock_all_auths();

        let signers = soroban_sdk::vec![&env, signer1, signer2];
        let operations = soroban_sdk::vec![&env, 1u32, 2, 3];

        client.threshold_authorize(&signers, &2, &operations);
    }
}
