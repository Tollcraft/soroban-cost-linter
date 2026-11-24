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

/// A `while let` header desugars to a `loop` node that encloses the body, so
/// the authorization inside it is still reported, once.
fn bad_same_address_auth_in_while_let_loop(env: Env) {
    let items = [Address, Address];
    let mut iter = items.iter();
    while let Some(item) = iter.next() {
        item.require_auth(); // Should Warn — authorization inside the loop body
    }
}

/// The authorization is hoisted above the `while let`, so the body only reads
/// the item. Silence here shows the anchor is the body, not the `while let`.
fn good_auth_before_while_let_loop(env: Env) {
    let addr = Address;
    addr.require_auth(); // Good — hoisted once, outside the loop
    let items = [Address, Address];
    let mut iter = items.iter();
    while let Some(item) = iter.next() {
        let _ = item; // only reads the item inside the loop
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

// --- Positive: the UFCS spelling is recognized exactly like the method call ---

/// `Address::require_auth(&addr)` is the same authorization as
/// `addr.require_auth()`, written in universal-function-call syntax. It must
/// be flagged in a loop for the same reason.
fn bad_ufcs_auth_in_loop(env: Env) {
    let addr = Address;
    for _ in 0..10 {
        Address::require_auth(&addr); // Should Warn — UFCS form in a loop
    }
}

/// UFCS `require_auth_for_args` in a loop, mirroring the method-call case.
fn bad_ufcs_auth_for_args_in_loop(env: Env) {
    let addr = Address;
    for i in 0..10 {
        Address::require_auth_for_args(&addr, &[i]); // Should Warn
    }
}

// --- Negative: test-only code is exempt (harness compiles without --test) ---

#[cfg(test)]
fn test_helper_auth_in_loop(env: Env) {
    let addr = Address;
    for _ in 0..10 {
        addr.require_auth(); // Good — test helper, not a metered host call
    }
}

#[cfg(test)]
mod tests {
    use super::Address;

    #[test]
    fn test_module_auth_in_loop() {
        let addr = Address;
        for _ in 0..10 {
            addr.require_auth(); // Good — test module, not a metered host call
        }
    }
}

// --- Negative (should not warn): authorization outside a loop ---

fn good_auth_outside_loop(env: Env) {
    let addr = Address;
    addr.require_auth(); // Good — called once
}

/// The UFCS spelling outside a loop is just as harmless as the method call.
fn good_ufcs_auth_outside_loop(env: Env) {
    let addr = Address;
    Address::require_auth(&addr); // Good — not inside a loop
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

// --- Negative: closures and iterator adapters are out of scope ---

fn good_for_each_closure(env: Env, addrs: [Address; 5]) {
    addrs.iter().for_each(|a| a.require_auth()); // Good — closure, not a loop body
}

// The hoisted equivalent is equally silent: the receiver is out of reach when
// the closure is defined away from the iterator that invokes it.
fn good_hoisted_closure(env: Env, addrs: [Address; 5]) {
    let authorize = |a: &Address| a.require_auth();
    addrs.iter().for_each(authorize);
}

fn main() {}
