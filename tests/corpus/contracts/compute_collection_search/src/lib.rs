#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Map, Symbol, Vec};

#[contract]
pub struct ComputeCollectionSearchContract;

#[contractimpl]
impl ComputeCollectionSearchContract {
    pub fn find_in_collection(_env: Env, keys: Vec<Symbol>, map: Map<Symbol, i32>) -> i32 {
        let mut sum = 0i32;
        for key in keys.iter() {
            if map.contains_key(key.clone()) {
                if let Some(val) = map.get(key) {
                    sum += val;
                }
            }
        }
        sum
    }
}
