#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, IntoVal, Address, Env, Vec};

#[contract]
pub struct CrossContractCallLoopContract;

#[contractimpl]
impl CrossContractCallLoopContract {
    pub fn batch_transfer(env: Env, target: Address, recipients: Vec<Address>, amounts: Vec<i128>) {
        let mut i = 0;
        while i < recipients.len() {
            let recipient = recipients.get(i).unwrap();
            let amount = amounts.get(i).unwrap();
            let _: () = env.invoke_contract(&target, &symbol_short!("transfer"), (recipient, amount).into_val(&env));
            i += 1;
        }
    }
}
