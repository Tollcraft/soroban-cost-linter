pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env {
        fn clone(&self) -> Self { Env }
    }
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

    // Minimal stand-ins for the SDK's unbounded container types. Only the
    // shape needed for type resolution matters here — these fixtures never
    // execute.
    pub struct Vec;
    impl Vec {
        pub fn new() -> Vec { Vec }
    }

    pub struct Map;
    impl Map {
        pub fn new() -> Map { Map }
    }

    pub struct Bytes;
    impl Bytes {
        pub fn new() -> Bytes { Bytes }
    }
}

use soroban_sdk::{Bytes, Env, Map, Vec};

// =======================================================================
// instance_storage_for_unbounded_data — Fixtures
//
// Every write below is paired with a matching `.get()` on the same
// receiver/key so that the unrelated `storage_write_without_read` lint
// stays quiet and each `.stderr` line is attributable to this lint.
// =======================================================================

fn bad_vec_in_instance_storage(env: Env) {
    let _existing: Option<Vec> = env.storage().instance().get(&1u32);
    let items: Vec = Vec::new();
    env.storage().instance().set(&1u32, &items); // Should Warn
}

fn bad_map_in_instance_storage(env: Env) {
    let _existing: Option<Map> = env.storage().instance().get(&2u32);
    let entries: Map = Map::new();
    env.storage().instance().set(&2u32, &entries); // Should Warn
}

fn bad_bytes_in_instance_storage(env: Env) {
    let _existing: Option<Bytes> = env.storage().instance().get(&3u32);
    let blob: Bytes = Bytes::new();
    env.storage().instance().set(&3u32, &blob); // Should Warn
}

// Scalars, small fixed-size, and config-shaped values are not unbounded —
// they resolve to a plain, statically sized type rather than to
// `soroban_sdk::{Vec,Map,Bytes}`, so the lint does not fire.
struct Config {
    max_supply: u32,
    admin_set: bool,
}

fn good_scalar_in_instance_storage(env: Env) {
    let _existing: Option<u32> = env.storage().instance().get(&4u32);
    env.storage().instance().set(&4u32, &42u32); // Good: scalar
}

fn good_fixed_array_in_instance_storage(env: Env) {
    let _existing: Option<[u8; 32]> = env.storage().instance().get(&5u32);
    let fixed: [u8; 32] = [0; 32];
    env.storage().instance().set(&5u32, &fixed); // Good: fixed-size array
}

fn good_config_struct_in_instance_storage(env: Env) {
    let _existing: Option<Config> = env.storage().instance().get(&6u32);
    let cfg = Config { max_supply: 1_000_000, admin_set: true };
    env.storage().instance().set(&6u32, &cfg); // Good: config-shaped struct
}

// Same unbounded value type, but on persistent storage (keyed per entry) —
// this lint is specific to `.instance()`, so persistent writes never fire.
fn good_vec_in_persistent_storage(env: Env) {
    let _existing: Option<Vec> = env.storage().persistent().get(&7u32);
    let items: Vec = Vec::new();
    env.storage().persistent().set(&7u32, &items); // Good: persistent, not instance
}

#[allow(instance_storage_for_unbounded_data)]
fn allowed_vec_in_instance_storage(env: Env) {
    let _existing: Option<Vec> = env.storage().instance().get(&8u32);
    let items: Vec = Vec::new();
    env.storage().instance().set(&8u32, &items); // Good (allowed)
}

fn main() {}
