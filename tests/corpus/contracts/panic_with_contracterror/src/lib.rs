#![no_std]
use soroban_sdk::{contracterror, contractimpl, panic_with_error, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    Bad = 1,
}

pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn safe_error(env: Env) {
        panic_with_error!(env, Error::Bad);
    }
}