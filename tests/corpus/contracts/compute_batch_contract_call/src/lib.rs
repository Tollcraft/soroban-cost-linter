#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, IntoVal, Address, Env, Vec};

#[contract]
pub struct ComputeBatchContractCallContract;

#[contractimpl]
impl ComputeBatchContractCallContract {
    pub fn process_batch(env: Env, target: Address, amounts: Vec<i128>) {
        for amount in amounts.iter() {
            let _: () = env.invoke_contract(&target, &symbol_short!("process"), (amount,).into_val(&env));
        }
    }
}
