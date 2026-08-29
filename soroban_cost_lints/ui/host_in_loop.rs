pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env {
        fn clone(&self) -> Self { Env }
    }
    impl Env {
        pub fn host(&self) -> host::Host {
            host::Host
        }
    }

    pub mod host {
        pub struct Host;
        impl Host {
            pub fn invoke_contract(&self) {}
            pub fn budget_cloned(&self) {}
        }
    }
}

use soroban_sdk::Env;

// =======================================================================
// host_in_loop — Fixtures
// =======================================================================

fn bad_host_in_for_loop(env: Env) {
    for _ in 0..10 {
        env.host().invoke_contract(); // Should Warn
    }
}

fn bad_host_in_while_loop(env: Env) {
    let mut i = 0;
    while i < 10 {
        env.host().budget_cloned(); // Should Warn
        i += 1;
    }
}

fn bad_host_in_loop_loop(env: Env) {
    loop {
        env.host().invoke_contract(); // Should Warn
        break;
    }
}

fn bad_host_in_iterator_closure(env: Env) {
    (0..3).for_each(|_| {
        env.host().invoke_contract(); // Should Warn
    });
}

fn good_host_outside_loop(env: Env) {
    env.host().invoke_contract(); // Good — outside any loop
}

fn good_host_in_option_map(env: Env) {
    let opt = Some(env);
    opt.map(|e| {
        e.host().invoke_contract(); // Good — Option::map calls at most once
    });
}

#[allow(host_in_loop)]
fn allowed_host_in_loop(env: Env) {
    for _ in 0..10 {
        env.host().invoke_contract(); // Good (allowed)
    }
}

fn main() {}
