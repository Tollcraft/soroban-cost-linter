#![allow(
    unused,
    storage_write_without_read,
    soroban_redundant_storage_read,
    persistent_read_without_ttl_extension,
    instance_storage_for_unbounded_data,
    loop_invariant_storage_access,
    soroban_storage_in_loop,
    unbounded_input_loop,
    unwrap_on_storage_get
)]

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
            pub fn extend_ttl<K>(&self, _k: &K, _threshold: &()) {}
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn extend_ttl<K>(&self, _k: &K, _threshold: u32, _extend_to: u32) {}
        }
    }
}

use soroban_sdk::Env;

// =======================================================================
//  Triggering cases — these MUST produce the lint warning
// =======================================================================

// Fires: write to temporary storage followed by unwrap on get of same key.
fn bad_set_then_unwrap(env: Env) -> i32 {
    env.storage().temporary().set(&1, &42);
    env.storage().temporary().get::<_, i32>(&1).unwrap() //~ WARNING unsafe read from temporary storage
}

// Fires: write to temporary storage followed by expect on get of same key.
fn bad_set_then_expect(env: Env) -> i32 {
    env.storage().temporary().set(&"key", &100);
    env.storage().temporary().get::<_, i32>(&"key").expect("must exist") //~ WARNING unsafe read from temporary storage
}

// =======================================================================
//  Non-triggering cases — these MUST NOT produce a warning
// =======================================================================

// Near-miss: unwrap_or provides a default when the temporary entry has expired.
fn good_set_then_unwrap_or(env: Env) -> i32 {
    env.storage().temporary().set(&1, &42);
    env.storage().temporary().get::<_, i32>(&1).unwrap_or(0)
}

// Near-miss: match handles the None case explicitly.
fn good_set_then_match(env: Env) -> i32 {
    env.storage().temporary().set(&1, &42);
    match env.storage().temporary().get::<_, i32>(&1) {
        Some(v) => v,
        None => 0,
    }
}

// Near-miss: an explicit existence check — the `has`-guarded plain `get`
// returns an `Option` (no `unwrap`/`expect`), so absence on TTL expiry is
// handled explicitly rather than assumed present.
fn good_set_then_has_then_get(env: Env) -> i32 {
    env.storage().temporary().set(&1, &42);
    let value: Option<i32> = if env.storage().temporary().has(&1) {
        env.storage().temporary().get(&1)
    } else {
        None
    };
    value.unwrap_or(0)
}

// Near-miss: no write precedes the read, so the get is a standalone lookup
// (handled by unwrap_on_storage_get instead).
fn good_get_without_prior_set(env: Env) -> i32 {
    env.storage().temporary().get::<_, i32>(&1).unwrap_or(0)
}

// Near-miss: the write is to a different key than the read.
fn good_different_keys(env: Env) -> i32 {
    env.storage().temporary().set(&1, &42);
    env.storage().temporary().get::<_, i32>(&2).unwrap_or(0)
}

// Near-miss: the write is to persistent storage, not temporary.
fn good_persistent_write_then_read(env: Env) -> i32 {
    env.storage().persistent().set(&1, &42);
    env.storage().persistent().get::<_, i32>(&1).unwrap_or(0)
}

// Near-miss: intentional cache — temporary data that can be recomputed.
#[allow(clippy::let_unit_value)]
fn good_intentional_cache(env: Env) -> i32 {
    env.storage().temporary().set(&1, &42);
    let cached: Option<i32> = env.storage().temporary().get(&1);
    cached.unwrap_or_else(|| {
        let recomputed = 42;
        env.storage().temporary().set(&1, &recomputed);
        recomputed
    })
}

fn main() {}
