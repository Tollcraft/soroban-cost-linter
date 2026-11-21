#![warn(deploy_in_loop)]
#![allow(unnecessary_host_function_call)]

pub mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn deployer(&self) -> deploy::Deployer {
            deploy::Deployer
        }
    }

    pub mod deploy {
        pub struct Deployer;
        impl Deployer {
            pub fn deploy(&self, _wasm_hash: [u8; 32]) {}
            pub fn deploy_v2(&self, _wasm_hash: [u8; 32]) {}
        }
    }
}
use soroban_sdk::Env;

fn bad_deploy(env: &Env) {
    for _ in 0..10 {
        env.deployer().deploy([0; 32]); //~ WARNING contract deployment inside a loop
    }
}

fn bad_deploy_v2(env: &Env) {
    for _ in 0..10 {
        env.deployer().deploy_v2([0; 32]); //~ WARNING contract deployment inside a loop
    }
}

fn good_deploy_outside_loop(env: &Env) {
    env.deployer().deploy([0; 32]);
    env.deployer().deploy_v2([0; 32]);
}

fn good_deployer_without_deploy(env: &Env) {
    for _ in 0..10 {
        let _ = env.deployer();
    }
}

#[allow(deploy_in_loop)]
fn allowed_factory_loop(env: &Env) {
    for _ in 0..10 {
        env.deployer().deploy([0; 32]);
    }
}

fn main() {}
