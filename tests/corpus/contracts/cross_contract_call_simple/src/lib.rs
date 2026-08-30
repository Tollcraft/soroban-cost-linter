#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, IntoVal, Address, Env};

#[contract]
pub struct CrossContractCallSimpleContract;

#[contractimpl]
impl CrossContractCallSimpleContract {
    pub fn call_external(env: Env, target: Address, amount: i128) -> i128 {
        let res: i128 = env.invoke_contract(&target, &symbol_short!("transfer"), (amount,).into_val(&env));
        res
    }
}
