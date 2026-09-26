pub mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn current_contract_address(&self) -> Address { Address }
    }

    pub struct Address;
    impl Address {
        pub fn require_auth(&self) {}
        pub fn require_auth_for_args(&self, _args: &[i32]) {}
    }
}

use soroban_sdk::{Address, Env};

// =======================================================================
// require_auth_in_loop — Fixtures
// =======================================================================

// --- Positive (should warn): same address authorized repeatedly in a loop ---

/// A `for` loop re-authorizing the same address on every iteration. The first
/// call already authorized the address for the whole invocation, so the rest
/// only repeat the host call.
fn bad_same_address_auth_in_for_loop(env: Env) {
    let addr = Address;
    for _ in 0..10 {
        addr.require_auth(); // Should Warn — same address every iteration
    }
}

/// Same as above with a `while` loop: the loop kind is irrelevant to the lint.
fn bad_same_address_auth_in_while_loop(env: Env) {
    let addr = Address;
    let mut i = 0;
    while i < 10 {
        addr.require_auth(); // Should Warn — same address every iteration
        i += 1;
    }
}

/// A bare `loop` with a manual break, to cover all three loop kinds.
fn bad_same_address_auth_in_loop_loop(env: Env) {
    let addr = Address;
    let mut count = 0;
    loop {
        addr.require_auth(); // Should Warn — same address every iteration
        count += 1;
        if count >= 5 {
            break;
        }
    }
}

/// `require_auth_for_args` is tracked exactly like `require_auth`.
fn bad_require_auth_for_args_in_loop(env: Env) {
    let addr = Address;
    for i in 0..10 {
        addr.require_auth_for_args(&[i]); // Should Warn
    }
}

/// `require_auth_for_args` in a `while` loop.
fn bad_require_auth_for_args_in_while_loop(env: Env) {
    let addr = Address;
    let mut i = 0;
    while i < 10 {
        addr.require_auth_for_args(&[i]); // Should Warn
        i += 1;
    }
}

/// An authorization nested in an inner loop is still inside a loop.
fn bad_nested_loop_auth(env: Env) {
    let addr = Address;
    for _ in 0..3 {
        for _ in 0..3 {
            addr.require_auth(); // Should Warn
        }
    }
}

// --- Known false positive: distinct per-iteration addresses still fire ---

// The lint does not check whether the address depends on the loop variable.
// When each iteration authorizes a genuinely distinct address, the
// per-iteration auth is intentional and should not be flagged.
fn good_distinct_address_per_iteration(env: Env, addrs: [Address; 5]) {
    for addr in addrs.iter() {
        addr.require_auth(); // False positive — each iteration authorizes a different address
    }
}

// --- Negative (should not warn): authorization outside a loop ---

fn good_auth_outside_loop(env: Env) {
    let addr = Address;
    addr.require_auth(); // Good — called once
}

// --- Negative: a receiver that is not a soroban_sdk::Address is out of scope ---

struct NotAnAddress;
impl NotAnAddress {
    fn require_auth(&self) {}
}

fn good_require_auth_on_other_type(env: Env) {
    let other = NotAnAddress;
    for _ in 0..10 {
        other.require_auth(); // Good — not a soroban_sdk::Address
    }
}

// --- Suppression test ---

#[allow(require_auth_in_loop)]
fn allowed_auth_in_loop(env: Env) {
    let addr = Address;
    for _ in 0..10 {
        addr.require_auth(); // Good (allowed)
    }
}

fn main() {}
