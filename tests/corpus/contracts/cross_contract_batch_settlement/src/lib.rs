#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, IntoVal, Address, Env, Vec};

#[contract]
pub struct CrossContractBatchSettlementContract;

#[contractimpl]
impl CrossContractBatchSettlementContract {
    pub fn settle_batch(env: Env, token: Address, recipients: Vec<Address>, amounts: Vec<i128>) {
        let mut i = 0;
        while i < recipients.len() && i < amounts.len() {
            let recipient = recipients.get(i).unwrap();
            let amount = amounts.get(i).unwrap();
            let _: () = env.invoke_contract(
                &token,
                &symbol_short!("transfer"),
                (recipient, amount).into_val(&env),
            );
            i += 1;
        }
    }
}
