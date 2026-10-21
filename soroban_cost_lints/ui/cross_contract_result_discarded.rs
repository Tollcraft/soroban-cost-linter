// UI test suite for the `cross_contract_result_discarded` lint rule.
//
// Each test case is self-contained with a minimal mock of `soroban_sdk::Env`
// so the file compiles without the real Soroban SDK dependency. `invoke_contract`
// mirrors the structural shape recognized by the lint: a method named
// `invoke_contract` on `Env` that returns a generic `T`.

pub mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn invoke_contract<T>(&self, _contract: &Address, _func: &Symbol, _args: ()) -> T
        where
            T: Default,
        {
            T::default()
        }
    }
    pub struct Address;
    pub struct Symbol;
}

use soroban_sdk::{Address, Env, Symbol};

// ===========================================================================
//  Triggering cases — these MUST produce the cross_contract_result_discarded warning
// ===========================================================================

/// Result bound to the wildcard `_` is discarded.
fn bad_let_underscore_discards(env: Env, addr: Address, func: Symbol) {
    let _ = env.invoke_contract::<i32>(&addr, &func, ()); //~ WARNING cross-contract call result discarded
}

/// Call dropped as a bare statement; result is discarded.
fn bad_semi_statement_discards(env: Env, addr: Address, func: Symbol) {
    env.invoke_contract::<i32>(&addr, &func, ()); //~ WARNING cross-contract call result discarded
}

// ===========================================================================
//  Non-triggering cases — these MUST NOT produce a warning
// ===========================================================================

/// Result bound to a named variable is considered used.
fn good_result_bound_to_named(env: Env, addr: Address, func: Symbol) {
    let result = env.invoke_contract::<i32>(&addr, &func, ());
    let _ = result;
}

/// Result consumed as an argument is considered used.
fn good_result_used_as_arg(env: Env, addr: Address, func: Symbol) {
    let _ = takes(env.invoke_contract::<i32>(&addr, &func, ()));
}

/// A call whose result type is the unit type `()` has nothing to discard
/// (bound to `_`).
fn good_unit_result_wildcard(env: Env, addr: Address, func: Symbol) {
    let _ = env.invoke_contract::<()>(&addr, &func, ());
}

/// A call whose result type is the unit type `()` has nothing to discard
/// (dropped as a statement).
fn good_unit_result_semi(env: Env, addr: Address, func: Symbol) {
    env.invoke_contract::<()>(&addr, &func, ());
}

/// Binding to a named variable with an underscore prefix is the conventional
/// way to deliberately silence a "must use" warning; the result is bound, so
/// the lint does not fire.
fn good_deliberate_silence_with_named(env: Env, addr: Address, func: Symbol) {
    let _result = env.invoke_contract::<i32>(&addr, &func, ());
}

/// Explicitly allowed via attribute.
#[allow(cross_contract_result_discarded)]
fn allowed_discard(env: Env, addr: Address, func: Symbol) {
    let _ = env.invoke_contract::<i32>(&addr, &func, ());
}

fn takes(_: i32) {}

fn main() {}
