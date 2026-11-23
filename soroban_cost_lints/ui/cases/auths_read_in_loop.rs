#![warn(auths_read_in_loop)]

pub mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn auths(&self) {}
    }
}

use soroban_sdk::Env;

struct LocalStub;
impl LocalStub {
    pub fn auths(&self) {}
}

fn bad_auths_in_for_loop(env: &Env) {
    for _ in 0..10 {
        env.auths(); //~ WARNING calling env.auths() inside a loop
    }
}

fn bad_auths_in_while_loop(env: &Env) {
    let mut i = 0;
    while i < 10 {
        env.auths(); //~ WARNING calling env.auths() inside a loop
        i += 1;
    }
}

fn bad_auths_in_loop_loop(env: &Env) {
    let mut i = 0;
    loop {
        env.auths(); //~ WARNING calling env.auths() inside a loop
        i += 1;
        if i >= 10 {
            break;
        }
    }
}

fn good_auths_outside_loop(env: &Env) {
    env.auths();
}

fn good_auths_hoisted_above_loop(env: &Env) {
    let _auths = env.auths();
    for _ in 0..10 {
        let _ = &_auths;
    }
}

fn good_stub_auths_in_loop(stub: &LocalStub) {
    for _ in 0..10 {
        stub.auths();
    }
}

fn good_auths_in_closure_in_loop(env: &Env) {
    for _ in 0..10 {
        let _closure = || {
            env.auths();
        };
    }
}

#[allow(auths_read_in_loop)]
fn allowed_auths_in_loop(env: &Env) {
    for _ in 0..10 {
        env.auths();
    }
}

fn main() {}
