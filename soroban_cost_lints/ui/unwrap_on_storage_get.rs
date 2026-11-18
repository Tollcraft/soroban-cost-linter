#![allow(
    storage_write_without_read,
    persistent_read_without_ttl_extension,
    instance_storage_for_unbounded_data,
    loop_invariant_storage_access,
    soroban_storage_in_loop,
    unbounded_input_loop
)]

pub mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn storage(&self) -> storage::Storage {
            storage::Storage
        }
    }

    pub mod storage {
        pub struct Storage;
        impl Storage {
            pub fn instance(&self) -> Instance { Instance }
            pub fn persistent(&self) -> Persistent { Persistent }
            pub fn temporary(&self) -> Temporary { Temporary }
        }

        pub struct Instance;
        impl Instance {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
        }
    }
}

use soroban_sdk::Env;

// ===========================================================================
//  Triggering cases — these MUST produce the unwrap_on_storage_get warning
// ===========================================================================

// Fires: `.unwrap()` directly on an instance-storage read.
fn bad_instance_unwrap(env: Env) -> i32 {
    env.storage().instance().get::<_, i32>(&1).unwrap() //~ WARNING unwrap on a storage read traps the contract
}

// Fires: `.unwrap()` directly on a persistent-storage read.
fn bad_persistent_unwrap(env: Env) -> i32 {
    env.storage().persistent().get::<_, i32>(&1).unwrap() //~ WARNING unwrap on a storage read traps the contract
}

// Fires: `.expect()` is the same panic-on-None shape as `.unwrap()`.
fn bad_temporary_expect(env: Env) -> i32 {
    env.storage()
        .temporary()
        .get::<_, i32>(&1)
        .expect("key must exist") //~ WARNING unwrap on a storage read traps the contract
}

// ===========================================================================
//  Non-triggering cases — these MUST NOT produce a warning
// ===========================================================================

// Near-miss: the read's Option is matched explicitly, so a missing key takes
// a proper error path instead of trapping.
fn good_match(env: Env) -> i32 {
    match env.storage().instance().get::<_, i32>(&1) {
        Some(v) => v,
        None => 0, // Should NOT warn
    }
}

// Near-miss: `unwrap_or` supplies a default instead of panicking.
fn good_unwrap_or(env: Env) -> i32 {
    env.storage().instance().get::<_, i32>(&1).unwrap_or(0) // Should NOT warn
}

// Near-miss: `unwrap_or_else` computes a default instead of panicking.
fn good_unwrap_or_else(env: Env) -> i32 {
    env.storage()
        .persistent()
        .get::<_, i32>(&1)
        .unwrap_or_else(|| 0) // Should NOT warn
}

// Out of scope: `unwrap` on an Option that did not come from a storage read.
fn good_non_storage_option(value: Option<i32>) -> i32 {
    value.unwrap() // Should NOT warn
}

// Out of scope: unwrapping something that is not a `get` at all.
fn good_non_get_unwrap(env: Env) {
    let _ = env; // Should NOT warn
}

// ===========================================================================
//  Test code — none of the flagged patterns above should fire under
//  `#[cfg(test)]`, whether on the function directly or on an enclosing
//  `mod tests { .. }`.
// ===========================================================================

#[cfg(test)]
fn cfg_test_fn_unwrap(env: Env) -> i32 {
    env.storage().instance().get::<_, i32>(&1).unwrap()
}

#[cfg(test)]
mod tests {
    use super::soroban_sdk::Env;

    fn in_test_module_unwrap(env: Env) -> i32 {
        env.storage().instance().get::<_, i32>(&1).unwrap()
    }
}

fn main() {}
