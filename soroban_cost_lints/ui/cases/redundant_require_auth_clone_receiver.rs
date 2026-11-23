#![allow(unused)]
#![warn(redundant_require_auth)]

mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn invoke_contract(&self) {}
    }

    #[derive(Clone)]
    pub struct Address;
    impl Address {
        pub fn require_auth(&self) {}
        pub fn require_auth_for_args(&self, _args: &[i32]) {}
    }
}

use soroban_sdk::{Address, Env};

fn bad_clone_receiver(env: Env) {
    let addr = Address;
    addr.require_auth();
    addr.clone().require_auth();
}

fn bad_ref_receiver(env: Env) {
    let addr = Address;
    addr.require_auth();
    (&addr).require_auth();
}

fn good_distinct_addresses(env: Env) {
    let addr1 = Address;
    let addr2 = Address;
    addr1.require_auth();
    addr2.require_auth();
}

fn good_invoke_contract_resets(env: Env) {
    let addr = Address;
    addr.require_auth();
    env.invoke_contract();
    addr.clone().require_auth();
}

fn main() {}
