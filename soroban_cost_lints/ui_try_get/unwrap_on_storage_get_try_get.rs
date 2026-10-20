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
            pub fn try_get<K, V>(&self, _k: &K) -> Option<V> { None }
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn try_get<K, V>(&self, _k: &K) -> Option<V> { None }
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn try_get<K, V>(&self, _k: &K) -> Option<V> { None }
        }
    }
}

use soroban_sdk::Env;

// Fires: `.unwrap()` directly on a persistent-storage `try_get`.
fn bad_try_get_unwrap(env: Env) -> i32 {
    env.storage().persistent().try_get::<_, i32>(&1).unwrap() //~ WARNING unwrap on a storage read traps the contract
}

// Fires: `.expect()` is the same panic-on-None shape.
fn bad_try_get_expect(env: Env) -> i32 {
    env.storage()
        .instance()
        .try_get::<_, i32>(&1)
        .expect("key must exist") //~ WARNING unwrap on a storage read traps the contract
}

// Near-miss: `unwrap_or` supplies a default instead of panicking.
fn good_try_get_unwrap_or(env: Env) -> i32 {
    env.storage().temporary().try_get::<_, i32>(&1).unwrap_or(0) // Should NOT warn
}

// Near-miss: the read's Option is matched explicitly.
fn good_try_get_match(env: Env) -> i32 {
    match env.storage().instance().try_get::<_, i32>(&1) {
        Some(v) => v,
        None => 0, // Should NOT warn
    }
}

#[cfg(test)]
mod tests {
    use super::soroban_sdk::Env;

    fn in_test_module_unwrap(env: Env) -> i32 {
        env.storage().instance().try_get::<_, i32>(&1).unwrap()
    }
}

fn main() {}
