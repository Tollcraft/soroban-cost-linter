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

/// Same address, `require_auth` called twice in one function. The second call
/// adds no authorization; only it is reported, so the author sees the line to
/// delete.
fn bad_double_require_auth(env: Env) {
    let addr = Address;
    addr.require_auth(); // first — no warning
    addr.require_auth(); //~ WARNING require_auth already called on this address
}

/// Same address, `require_auth_for_args` called twice.
fn bad_double_require_auth_for_args(env: Env) {
    let addr = Address;
    addr.require_auth_for_args(&[1]); // first — no warning
    addr.require_auth_for_args(&[2]); //~ WARNING require_auth already called on this address
}

/// Same address, mixed `require_auth` and `require_auth_for_args`. Both are
/// tracked as authorization of the same address, so the second fires.
fn bad_mixed_auth_methods(env: Env) {
    let addr = Address;
    addr.require_auth(); //~ WARNING require_auth already called on this address
    addr.require_auth_for_args(&[1, 2]); //~ WARNING require_auth already called on this address
}

/// Second call fires, first does not — pins which span is reported.
fn bad_second_fires(env: Env) {
    let addr = Address;
    addr.require_auth(); // first — no warning
    addr.require_auth(); //~ WARNING require_auth already called on this address
}

/// Sibling blocks of one function share one authorization scope: the first
/// block authorizes the address, so the second block's call is redundant.
fn bad_sibling_blocks(env: Env, choose: bool) {
    let addr = Address;
    if choose {
        addr.require_auth(); // first — no warning
    } else {
        addr.require_auth(); //~ WARNING require_auth already called on this address
    }
}

/// The same, with plain nested blocks instead of an if/else.
fn bad_sequential_nested_blocks(env: Env) {
    let addr = Address;
    {
        addr.require_auth(); // first — no warning
    }
    {
        addr.require_auth(); //~ WARNING require_auth already called on this address
    }
}

// =======================================================================
// Non-triggering cases — these MUST NOT produce any warning
// =======================================================================

/// Two genuinely different addresses: neither is authorized twice.
fn good_different_addresses() {
    let addr_a = Address;
    let addr_b = Address;
    addr_a.require_auth();
    addr_b.require_auth();
}

/// Same address but separated by a cross-contract call. `invoke_contract`
/// resets the tracking, because authorization state across a callee cannot be
/// reasoned about here.
fn good_separated_by_invoke_contract(env: Env, target: Address, func: Symbol) {
    let addr = Address;
    addr.require_auth();
    let _: () = env.invoke_contract(&target, &func, ());
    addr.require_auth();
}

/// Same address but separated by a `try_invoke_contract` call.
///
/// The result is discarded deliberately — this fixture pins how
/// `redundant_require_auth` resets across a fallible cross-contract call, not
/// whether the call's value is used.
#[allow(cross_contract_result_discarded)]
fn good_separated_by_try_invoke_contract(env: Env, target: Address, func: Symbol) {
    let addr = Address;
    addr.require_auth();
    let _: Result<(), ()> = env.try_invoke_contract(&target, &func, ());
    addr.require_auth();
}

/// Single `require_auth` — no duplication.
fn good_single_require_auth() {
    let addr = Address;
    addr.require_auth();
}

/// Two functions are two scopes: an authorization established in one says
/// nothing about the other, even for the same syntactic address.
fn good_across_functions_a(env: Env) {
    let addr = Address;
    addr.require_auth();
}

fn good_across_functions_b(env: Env) {
    let addr = Address;
    addr.require_auth(); // Good — a different function
}

/// No `require_auth` calls at all.
fn good_no_require_auth() {
    let _env = Env;
}

struct AddressBook;

impl AddressBook {
    fn require_auth(&self) {}
}

fn good_user_address_book() {
    let book = AddressBook;
    book.require_auth();
    book.require_auth();
}

struct MockEnv;

impl MockEnv {
    fn invoke_contract(&self) {}
}

fn bad_user_mock_env_does_not_reset_tracking() {
    let env = MockEnv;
    let addr = Address;
    addr.require_auth();
    env.invoke_contract();
    addr.require_auth(); //~ WARNING require_auth already called on this address
}

fn main() {}
