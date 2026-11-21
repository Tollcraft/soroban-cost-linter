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

    /// Stand-in for `soroban_sdk::Map`: a real collection whose `get` returns
    /// `Option`, but which is not a storage accessor.
    pub struct Map<K, V>(core::marker::PhantomData<(K, V)>);
    impl<K, V> Map<K, V> {
        pub fn get(&self, _k: &K) -> Option<V> { None }
    }
}

use soroban_sdk::{Env, Map};

// A user type with its own `get`; it must not be mistaken for storage.
struct Registry;
impl Registry {
    fn get(&self) -> Option<u32> { None }
}

// Near-miss: `Map::get(..).unwrap()` is a collection read, not a storage read.
fn good_map_get_unwrap(map: Map<u32, u32>) -> u32 {
    map.get(&1).unwrap() // Should NOT warn
}

// Near-miss: the same through a lookup helper.
fn lookup(map: &Map<u32, u32>, key: u32) -> Option<u32> {
    map.get(&key)
}

fn good_helper_unwrap(map: Map<u32, u32>) -> u32 {
    lookup(&map, 1).unwrap() // Should NOT warn
}

// Near-miss: a user-defined `get` returning `Option`.
fn good_registry_get_unwrap(registry: Registry) -> u32 {
    registry.get().unwrap() // Should NOT warn
}

// Suppression path: the documented `#[allow]` is accepted.
#[allow(unwrap_on_storage_get)]
fn allowed_unwrap(map: Map<u32, u32>) -> u32 {
    map.get(&1).unwrap() // Good (allowed)
}

fn main() {
    let _ = Env;
}
