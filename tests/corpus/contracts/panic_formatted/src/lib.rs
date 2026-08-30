#![no_std]
use soroban_sdk::contractimpl;

pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn trigger(x: u32) {
        panic!("formatted panic: {}", x);
    }
}