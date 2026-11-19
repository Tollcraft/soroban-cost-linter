#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol, Vec};

const LIMIT_KEY: Symbol = symbol_short!("limit");

#[contract]
pub struct ComputeInputValidationContract;

#[contractimpl]
impl ComputeInputValidationContract {
    pub fn validate_inputs(env: Env, inputs: Vec<i32>) -> i32 {
        let mut total = 0i32;
        for val in inputs.iter() {
            let max_limit: i32 = env.storage().instance().get(&LIMIT_KEY).unwrap_or(100);
            if val > max_limit {
                total += max_limit;
            } else {
                total += val;
            }
        }
        total
    }

    pub fn validate_bounded(env: Env, inputs: Vec<i32>) -> i32 {
        let max_limit: i32 = env.storage().instance().get(&LIMIT_KEY).unwrap_or(100);
        let mut total = 0i32;
        let len = inputs.len().min(10);
        for i in 0..len {
            if let Some(val) = inputs.get(i) {
                if val > max_limit {
                    total += max_limit;
                } else {
                    total += val;
                }
            }
        }
        total
    }
}
