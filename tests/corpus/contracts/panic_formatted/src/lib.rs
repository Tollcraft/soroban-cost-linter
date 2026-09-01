#![no_std]
use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn trigger(x: u32) {
        panic!("formatted panic: {}", x);
    }
}
