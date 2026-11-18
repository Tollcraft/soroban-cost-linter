#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, IntoVal, Address, Env, Vec};

/// A contract that performs batch settlements by invoking a token contract 
/// to transfer funds to multiple recipients in a single transaction.
#[contract]
pub struct CrossContractBatchSettlementContract;

#[contractimpl]
impl CrossContractBatchSettlementContract {
    /// Settles a batch of token transfers by calling the `transfer` function 
    /// on the specified token contract for each recipient in the `recipients` list.
    /// 
    /// The number of transfers performed is the minimum of `recipients.len()` 
    /// and `amounts.len()`. This ensures that every recipient processed has a 
    /// corresponding amount and prevents out-of-bounds errors.
    /// 
    /// # Arguments
    /// 
    /// * `env` - The environment for the contract execution.
    /// * `token` - The address of the token contract to invoke.
    /// * `recipients` - A list of addresses representing the recipients.
    /// * `amounts` - A list of amounts to transfer, corresponding to each recipient.
    pub fn settle_batch(env: Env, token: Address, recipients: Vec<Address>, amounts: Vec<i128>) {
        let mut i = 0;
        
        // Iterate through the recipients and amounts. We use a while loop 
        // to safely process up to the length of the shorter vector.
        while i < recipients.len() && i < amounts.len() {
            let recipient = recipients.get(i).unwrap();
            let amount = amounts.get(i).unwrap();
            
            // Invoke the `transfer` function on the external token contract.
            // We use `invoke_contract` which dynamically calls the given token contract.
            let _: () = env.invoke_contract(
                &token,
                &symbol_short!("transfer"),
                (recipient, amount).into_val(&env),
            );
            i += 1;
        }
    }
}
