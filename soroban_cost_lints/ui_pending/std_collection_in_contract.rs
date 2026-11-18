#![feature(register_tool)]
#![register_tool(contractimpl)]

// UI test fixture for `std_collection_in_contract`.
//
// Firing cases: std collection usage inside a `#[contractimpl]` block.
// Non-firing cases: std collection usage outside contract code, or in test code.

pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env {
        fn clone(&self) -> Self {
            Env
        }
    }
}

use soroban_sdk::Env;
use std::collections::HashMap;

// =====================================================================
// Firing cases — std collections inside #[contractimpl]
// =====================================================================

pub struct MyContract;

#[contractimpl::impl_]
impl MyContract {
    fn bad_hashmap_in_method(_env: Env) {
        let mut _map: HashMap<String, i32> = HashMap::new(); // Should Warn
        let _val = _map.get("key"); // Should Warn
    }

    fn bad_vec_in_method(_env: Env) {
        let mut _vec: Vec<i32> = Vec::new(); // Should Warn
        _vec.push(42); // Should Warn
    }

    fn bad_btreemap_in_method(_env: Env) {
        let _map: std::collections::BTreeMap<u32, String> =
            std::collections::BTreeMap::new(); // Should Warn
    }

    fn bad_hashmap_with_capacity(_env: Env) {
        let _map: HashMap<String, i32> = HashMap::with_capacity(10); // Should Warn
    }

    fn bad_iterate_hashmap(_env: Env) {
        let mut map: HashMap<String, i32> = HashMap::new();
        map.insert("a".to_string(), 1);
        for (_key, _val) in map.iter() { // Should Warn
            // iterating over std collection
        }
    }
}

// =====================================================================
// Non-firing cases — std collections outside contract code
// =====================================================================

fn good_hashmap_outside_contract() {
    let mut _map: HashMap<String, i32> = HashMap::new(); // Good — not in contract code
    _map.insert("key".to_string(), 1);
}

fn good_vec_outside_contract() {
    let mut _vec: Vec<i32> = Vec::new(); // Good — not in contract code
    _vec.push(42);
}

struct PlainStruct;

impl PlainStruct {
    fn method_with_hashmap() {
        let _map: HashMap<String, i32> = HashMap::new(); // Good — not #[contractimpl]
    }
}

// =====================================================================
// Non-firing cases — std collections in test code
// =====================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    #[test]
    fn test_with_hashmap() {
        let mut map: HashMap<String, i32> = HashMap::new(); // Good — in test module
        map.insert("key".to_string(), 1);
    }
}

#[test]
fn test_with_vec() {
    let _vec: Vec<i32> = Vec::new(); // Good — test function
}

fn main() {}
