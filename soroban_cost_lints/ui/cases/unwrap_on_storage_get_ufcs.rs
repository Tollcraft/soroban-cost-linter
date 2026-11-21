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

use soroban_sdk::storage::Instance;
use soroban_sdk::Env;

// Fires: the UFCS spelling of an instance-storage read is the same trap as
// the method-call spelling.
fn bad_ufcs_instance_unwrap(env: Env) -> i32 {
    let storage = env.storage().instance();
    Instance::get::<_, i32>(&storage, &1).unwrap() //~ WARNING unwrap on a storage read traps the contract
}

// Fires: the method-call spelling is still reported.
fn bad_method_call_unwrap(env: Env) -> i32 {
    env.storage().instance().get::<_, i32>(&1).unwrap() //~ WARNING unwrap on a storage read traps the contract
}

// Near-miss: a `get` associated function on a non-storage type.
struct Registry;
impl Registry {
    fn get(&self, _k: &u32) -> Option<u32> {
        None
    }
}

fn good_non_storage_ufcs(registry: Registry) -> u32 {
    Registry::get(&registry, &1).unwrap() // Should NOT warn
}

// Near-miss: the UFCS read is matched rather than unwrapped.
fn good_ufcs_matched(env: Env) -> i32 {
    let storage = env.storage().instance();
    match Instance::get::<_, i32>(&storage, &1) {
        Some(v) => v,
        None => 0, // Should NOT warn
    }
}

fn main() {}
