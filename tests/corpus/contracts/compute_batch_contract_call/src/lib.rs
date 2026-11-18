#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, IntoVal, Vec};

#[contract]
pub struct ComputeBatchContractCallContract;

#[contractimpl]
impl ComputeBatchContractCallContract {
    /// Process a batch of amounts by invoking the target contract for each amount.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `target` - The address of the contract to invoke
    /// * `amounts` - A vector of amounts to process
    pub fn process_batch(env: Env, target: Address, amounts: Vec<i128>) {
        process_amounts(&env, &target, &amounts);
    }
}

/// Process each amount in the batch by invoking the target contract.
///
/// This helper function encapsulates the iteration and invocation logic,
/// making the code more modular and testable.
///
/// # Arguments
/// * `env` - The contract environment
/// * `target` - The address of the contract to invoke
/// * `amounts` - A vector of amounts to process
fn process_amounts(env: &Env, target: &Address, amounts: &Vec<i128>) {
    for amount in amounts.iter() {
        invoke_process_on_target(env, target, amount);
    }
}

/// Invoke the "process" function on the target contract with the given amount.
///
/// This helper function encapsulates the contract invocation logic,
/// separating concerns and improving testability.
///
/// # Arguments
/// * `env` - The contract environment
/// * `target` - The address of the contract to invoke
/// * `amount` - The amount to pass to the target contract's process function
fn invoke_process_on_target(env: &Env, target: &Address, amount: i128) {
    let _: () = env.invoke_contract(target, &symbol_short!("process"), (amount,).into_val(env));
}
