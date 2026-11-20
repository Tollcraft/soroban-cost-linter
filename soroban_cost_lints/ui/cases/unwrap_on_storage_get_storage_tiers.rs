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

use soroban_sdk::storage::Storage;
use soroban_sdk::Env;

// The shared stubs give `Storage` no accessor of its own, so add one to let
// the bare `Storage` type be exercised as a receiver.
impl Storage {
    pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
}

fn bad_storage(env: Env) -> i32 {
    env.storage().get::<_, i32>(&1).unwrap() //~ WARNING unwrap on a storage read traps the contract
}

fn bad_instance(env: Env) -> i32 {
    env.storage().instance().get::<_, i32>(&1).unwrap() //~ WARNING unwrap on a storage read traps the contract
}

fn bad_persistent(env: Env) -> i32 {
    env.storage().persistent().get::<_, i32>(&1).unwrap() //~ WARNING unwrap on a storage read traps the contract
}

fn bad_temporary(env: Env) -> i32 {
    env.storage().temporary().get::<_, i32>(&1).unwrap() //~ WARNING unwrap on a storage read traps the contract
}

// Silent: `has` is not a read of the value, and returns a `bool`.
fn good_has(env: Env) -> bool {
    env.storage().instance().has(&1) // Should NOT warn
}


fn main() {}

#[cfg(test)]
mod tests {
    use super::soroban_sdk::Env;

    fn in_test_module_unwrap(env: Env) -> i32 {
        env.storage().instance().get::<_, i32>(&1).unwrap()
    }
}
