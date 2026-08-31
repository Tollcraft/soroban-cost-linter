pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env {
        fn clone(&self) -> Self { Env }
    }
    impl Env {
        pub fn invoke_contract<T>(&self, _contract: &Address, _func: &Symbol, _args: ()) -> T
        where T: Default {
            T::default()
        }
        pub fn try_invoke_contract<T>(&self, _contract: &Address, _func: &Symbol, _args: ()) -> Result<T, ()>
        where T: Default {
            Ok(T::default())
        }
    }

    pub struct Address;
    impl Address {
        pub fn require_auth(&self) {}
        pub fn require_auth_for_args(&self, _args: &[i32]) {}
    }

    pub struct Symbol;
    impl Symbol {
        pub fn new(_env: &Env, _s: &str) -> Symbol { Symbol }
    }
}

use soroban_sdk::{Address, Env, Symbol};

// =======================================================================
// Triggering cases — these MUST produce the redundant_require_auth warning
// =======================================================================

/// Same address, require_auth called twice.
fn bad_double_require_auth(env: Env) {
    let addr = Address;
    addr.require_auth(); // first — no warning
    addr.require_auth(); //~ WARNING require_auth already called on this address
}

/// Same address, mixed require_auth and require_auth_for_args.
fn bad_mixed_auth_methods(env: Env) {
    let addr = Address;
    addr.require_auth(); //~ WARNING require_auth already called on this address
    addr.require_auth_for_args(&[1, 2]); //~ WARNING require_auth already called on this address
}

/// Second call fires, first does not.
fn bad_second_fires(env: Env) {
    let addr = Address;
    addr.require_auth(); // first — no warning
    addr.require_auth(); //~ WARNING require_auth already called on this address
}

// =======================================================================
// Non-triggering cases — these MUST NOT produce any warning
// =======================================================================

/// Two genuinely different addresses.
fn good_different_addresses() {
    let addr_a = Address;
    let addr_b = Address;
    addr_a.require_auth();
    addr_b.require_auth();
}

/// Same address but separated by a cross-contract call.
fn good_separated_by_invoke_contract(env: Env, target: Address, func: Symbol) {
    let addr = Address;
    addr.require_auth();
    let _: () = env.invoke_contract(&target, &func, ());
    addr.require_auth();
}

/// Same address but separated by a try_invoke_contract call.
fn good_separated_by_try_invoke_contract(env: Env, target: Address, func: Symbol) {
    let addr = Address;
    addr.require_auth();
    let _: Result<(), ()> = env.try_invoke_contract(&target, &func, ());
    addr.require_auth();
}

/// Single require_auth — no duplication.
fn good_single_require_auth() {
    let addr = Address;
    addr.require_auth();
}

/// No require_auth calls at all.
fn good_no_require_auth() {
    let _env = Env;
}

fn main() {}
