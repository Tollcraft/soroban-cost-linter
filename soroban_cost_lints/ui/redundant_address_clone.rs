// UI test suite for the `redundant_address_clone` lint rule.
//
// Each test case is self-contained with a minimal mock of `soroban_sdk::Address`
// so the file compiles without the real Soroban SDK dependency.

pub mod soroban_sdk {
    pub struct Address;
    impl Clone for Address {
        fn clone(&self) -> Self {
            Address
        }
    }

    pub struct MyStruct {
        pub addr: Address,
    }
}

use soroban_sdk::Address;

// ===========================================================================
//  Triggering cases — these MUST produce the redundant_address_clone warning
// ===========================================================================

/// Simple redundant clone: addr is owned, not used after the clone.
fn bad_clone_address(addr: Address) {
    let _cloned = addr.clone(); //~ WARNING redundant clone on Address object
}

/// Clone result passed directly to a function; original not used after.
fn bad_clone_address_passed_to_fn(addr: Address) {
    takes_addr(addr.clone()); //~ WARNING redundant clone on Address object
}

/// Clone inside a block expression; original not used after.
fn bad_clone_address_in_block(addr: Address) {
    let _cloned = {
        addr.clone() //~ WARNING redundant clone on Address object
    };
}

/// Clone result stored and original immediately shadowed (not used after).
fn bad_clone_address_shadow(addr: Address) {
    let addr = addr.clone(); //~ WARNING redundant clone on Address object
    let _ = addr;
}

// ===========================================================================
//  Non-triggering cases — these MUST NOT produce a warning
// ===========================================================================

/// Cloning a reference `&Address` produces an owned `Address`; this is legitimate.
fn good_addr_ref_clone(addr_ref: &Address) {
    let _cloned = addr_ref.clone();
}

/// Original `addr` is used after the clone on the next line.
fn good_addr_used_after_clone(addr: Address) {
    let _cloned = addr.clone();
    let _still_here = addr;
}

/// Clone on a struct field (non-local binding) — conservatively skipped.
fn good_non_local_receiver(s: soroban_sdk::MyStruct) {
    let _cloned = s.addr.clone();
}

/// Clone is explicitly allowed via attribute.
#[allow(redundant_address_clone)]
fn allowed_clone_addr(addr: Address) {
    let _cloned = addr.clone();
}

fn takes_addr(_a: Address) {}

fn main() {}
