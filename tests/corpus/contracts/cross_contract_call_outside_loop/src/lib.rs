#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, IntoVal, Address, Env};

#[contract]
pub struct CrossContractCallOutsideLoopContract;

#[contractimpl]
impl CrossContractCallOutsideLoopContract {
    pub fn transfer_single(env: Env, token: Address, to: Address, amount: i128) {
        Self::execute_transfer(&env, &token, &to, amount);
    }

    /// Helper function to encapsulate the cross-contract call logic
    fn execute_transfer(env: &Env, token: &Address, to: &Address, amount: i128) {
        let args = (to.clone(), amount).into_val(env);
        let _: () = env.invoke_contract(token, &symbol_short!("transfer"), args);
    }
}
