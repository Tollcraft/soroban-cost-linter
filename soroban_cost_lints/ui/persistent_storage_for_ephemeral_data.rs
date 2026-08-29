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
            pub fn remove<K>(&self, _k: &K) {}
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn remove<K>(&self, _k: &K) {}
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

// Fires: persistent write immediately removed without condition. The entry
// never outlives the call, so `Persistent` only costs rent.
fn bad_set_then_remove(env: Env) {
    let key = 1i32;
    env.storage().persistent().set(&key, &42); //~ WARNING persistent storage write is removed on every path
    env.storage().persistent().remove(&key);
}

// Fires: the remove is on every arm of an if/else, so both paths tear the
// entry down; the write is still pure scratch space.
fn bad_remove_on_all_branches(env: Env, c: bool) {
    let key = 1i32;
    env.storage().persistent().set(&key, &42); //~ WARNING persistent storage write is removed on every path
    if c {
        env.storage().persistent().remove(&key);
    } else {
        env.storage().persistent().remove(&key);
    }
}

// =======================================================================
//  Non-triggering cases — these MUST NOT produce a warning
// =======================================================================

// Near-miss: removal is unconditional but of a different key than the write.
fn good_remove_of_different_key(env: Env) {
    let key = 1i32;
    env.storage().persistent().set(&key, &42);
    env.storage().persistent().remove(&2i32);
}

// Near-miss: removal happens only when the entry is overwritten — a classic
// `write(key); if overwrite(key) { read-or-delete }` pattern where the entry
// legitimately survives across calls.
fn good_remove_on_one_branch(env: Env, overwrite: bool) {
    let key = 1i32;
    env.storage().persistent().set(&key, &42);
    if overwrite {
        env.storage().persistent().remove(&key);
    }
}

// Near-miss: an early `return` can skip the removal, so the write is not
// provably removed on every path and must not be flagged.
fn good_early_return_skips_remove(env: Env, c: bool) {
    let key = 1i32;
    env.storage().persistent().set(&key, &42);
    if c {
        env.storage().persistent().remove(&key);
        return;
    }
    let _ = env.storage().persistent().get::<_, i32>(&key);
}

// Near-miss: the value survives to function exit — nobody removes it.
fn good_survives_to_exit(env: Env) -> i32 {
    let key = 1i32;
    env.storage().persistent().set(&key, &42);
    env.storage().persistent().get::<_, i32>(&key).unwrap_or(0)
}

// Near-miss: removal is gated by a guard flag — the author deliberately keeps
// the entry in the common case.
fn good_guard_flag(env: Env, cleanup: bool) {
    let key = 1i32;
    env.storage().persistent().set(&key, &42);
    if cleanup {
        env.storage().persistent().remove(&key);
    }
    let _ = env.storage().persistent().get::<_, i32>(&key);
}

// Near-miss: a loop may not run at all, so the removal inside it is not
// guaranteed on every path.
fn good_remove_inside_loop(env: Env, n: u32) {
    let key = 1i32;
    env.storage().persistent().set(&key, &42);
    for _ in 0..n {
        if env.storage().persistent().has(&key) {
            env.storage().persistent().remove(&key);
        }
    }
}

fn main() {}