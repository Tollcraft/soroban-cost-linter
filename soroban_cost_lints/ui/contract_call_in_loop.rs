pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env {
        fn clone(&self) -> Self { Env }
    }
    impl Env {
        pub fn current_contract_address(&self) -> Address {
            Address
        }
        pub fn invoke_contract<T>(&self, _contract: &Address, _func: &Symbol, _args: ()) -> T
        where
            T: Default,
        {
            T::default()
        }
        pub fn host(&self) -> host::Host {
            host::Host
        }
    }

    pub struct Address;
    impl Address {
        pub fn require_auth(&self) {}
    }

    pub struct Symbol;

    pub mod host {
        pub struct Host;
        impl Host {
            pub fn invoke_contract(&self) {}
        }
    }
}

use soroban_sdk::Env;

// =======================================================================
// contract_call_in_loop — Fixtures
// =======================================================================

fn bad_contract_call_in_for_loop(env: Env) {
    let addr = soroban_sdk::Address;
    let sym = soroban_sdk::Symbol;
    for _ in 0..10 {
        let _: () = env.invoke_contract(&addr, &sym, ()); // Should Warn
    }
}

fn bad_contract_call_in_while_loop(env: Env) {
    let addr = soroban_sdk::Address;
    let sym = soroban_sdk::Symbol;
    let mut i = 0;
    while i < 10 {
        let _: () = env.invoke_contract(&addr, &sym, ()); // Should Warn
        i += 1;
    }
}

fn bad_contract_call_in_iterator_closure(env: Env) {
    let addr = soroban_sdk::Address;
    let sym = soroban_sdk::Symbol;
    (0..3).for_each(|_| {
        let _: () = env.invoke_contract(&addr, &sym, ()); // Should Warn
    });
}

// Near-miss: the cross-contract call is loop-invariant (its result is reused),
// so it is hoisted out of the loop — only the in-loop call would fire. Here the
// call is entirely outside the loop, so nothing warns.
fn near_miss_invariant_call_hoisted_out(env: Env) {
    let addr = soroban_sdk::Address;
    let sym = soroban_sdk::Symbol;
    let shared: () = env.invoke_contract(&addr, &sym, ()); // Good — outside loop
    for _ in 0..10 {
        let _ = &shared; // Should not warn
    }
}

fn good_contract_call_outside_loop(env: Env) {
    let addr = soroban_sdk::Address;
    let sym = soroban_sdk::Symbol;
    let _: () = env.invoke_contract(&addr, &sym, ()); // Good — outside any loop
}

#[allow(contract_call_in_loop)]
fn allowed_contract_call_in_loop(env: Env) {
    let addr = soroban_sdk::Address;
    let sym = soroban_sdk::Symbol;
    for _ in 0..10 {
        let _: () = env.invoke_contract(&addr, &sym, ()); // Good (allowed)
    }
}

// Overlap: a single loop body that both invokes a contract (cross-contract
// call in loop) and uses a Host object (host in loop). Both lints fire on the
// same iteration.
fn overlapping_host_and_contract_call(env: Env) {
    let addr = soroban_sdk::Address;
    let sym = soroban_sdk::Symbol;
    for _ in 0..10 {
        let _: () = env.invoke_contract(&addr, &sym, ()); // Should Warn (contract_call_in_loop)
        env.host().invoke_contract(); // Should Warn (host_in_loop)
    }
}

fn main() {}
