#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, IntoVal, Address, Env};

#[contract]
pub struct CrossContractCallOutsideLoopContract;

#[contractimpl]
impl CrossContractCallOutsideLoopContract {
    pub fn transfer_single(env: Env, token: Address, to: Address, amount: i128) {
        let _: () = env.invoke_contract(&token, &symbol_short!("transfer"), (to, amount).into_val(&env));
    }
}
