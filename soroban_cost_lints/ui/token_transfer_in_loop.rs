#![allow(unused_variables)]
#![allow(redundant_env_clone)]

pub mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn clone(&self) -> Self { Env }
    }

    pub struct Address;
}

use soroban_sdk::{Address, Env};

struct TokenClient(Env);
impl TokenClient {
    pub fn transfer(&self, _from: &Address, _to: &Address, _amount: &i128) {}
    pub fn transfer_from(
        &self,
        _spender: &Address,
        _from: &Address,
        _to: &Address,
        _amount: &i128,
    ) {}
}

fn bad_transfer_in_for_loop(env: Env) {
    let client = TokenClient(env.clone());
    let from = Address;
    let to = Address;
    for _ in 0..10 {
        client.transfer(&from, &to, &100); // Should Warn
    }
}

fn bad_transfer_in_while_loop(env: Env) {
    let client = TokenClient(env.clone());
    let from = Address;
    let to = Address;
    let mut i = 0;
    while i < 10 {
        client.transfer(&from, &to, &100); // Should Warn
        i += 1;
    }
}

fn bad_transfer_in_loop_loop(env: Env) {
    let client = TokenClient(env.clone());
    let from = Address;
    let to = Address;
    loop {
        client.transfer(&from, &to, &100); // Should Warn
        break;
    }
}

fn bad_transfer_from_in_for_loop(env: Env) {
    let client = TokenClient(env.clone());
    let spender = Address;
    let from = Address;
    let to = Address;
    for _ in 0..10 {
        client.transfer_from(&spender, &from, &to, &50); // Should Warn
    }
}

fn bad_transfer_in_closure(env: Env) {
    let client = TokenClient(env.clone());
    let from = Address;
    let to = Address;
    (0..10).for_each(|_| {
        client.transfer(&from, &to, &100); // Should Warn — closure in loop
    });
}

fn good_transfer_outside_loop(env: Env) {
    let client = TokenClient(env.clone());
    let from = Address;
    let to = Address;
    client.transfer(&from, &to, &100); // Good — single call, not in a loop
}

fn good_transfer_from_outside_loop(env: Env) {
    let client = TokenClient(env.clone());
    let spender = Address;
    let from = Address;
    let to = Address;
    client.transfer_from(&spender, &from, &to, &50); // Good — outside loop
}

#[allow(token_transfer_in_loop)]
fn allowed_transfer_in_loop(env: Env) {
    let client = TokenClient(env.clone());
    let from = Address;
    let to = Address;
    for _ in 0..10 {
        client.transfer(&from, &to, &100); // Good (allowed)
    }
}

fn main() {}
