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
            pub fn extend_ttl(&self, _threshold: u32, _extend_to: u32) {}
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn extend_ttl<K: ?Sized>(&self, _key: &K, _threshold: u32, _extend_to: u32) {}
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn extend_ttl<K: ?Sized>(&self, _key: &K, _threshold: u32, _extend_to: u32) {}
        }
    }
}

use soroban_sdk::Env;

// Keep the metering parameters in one place so each fixture focuses on the
// loop shape it is meant to exercise. The lint should remain about repeated
// host calls, not about unrelated differences in threshold values.
const TTL_THRESHOLD: u32 = 100;
const TTL_EXTEND_TO: u32 = 1000;

// =======================================================================
// extend_ttl_in_loop — Fixtures
// =======================================================================

// `loop_invariant_storage_access` also matches these calls (any storage
// method call in a loop whose operands don't depend on loop state) — it's
// suppressed here so this fixture's `.stderr` stays focused on
// `extend_ttl_in_loop`'s own diagnostic; both lints firing on the same call
// is expected, not a bug (they recommend different fixes: hoist vs. batch).
#[allow(loop_invariant_storage_access)]
fn bad_instance_extend_ttl_in_for_loop(env: Env) {
    for _ in 0..10 {
        env.storage().instance().extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO); // Should Warn
    }
}

#[allow(loop_invariant_storage_access)]
fn bad_persistent_extend_ttl_in_while_loop(env: Env, keys: [u32; 3]) {
    let mut i = 0;
    while i < keys.len() {
        env.storage().persistent().extend_ttl(&keys[i], TTL_THRESHOLD, TTL_EXTEND_TO); // Should Warn
        i += 1;
    }
}

#[allow(loop_invariant_storage_access)]
fn bad_temporary_extend_ttl_in_loop_loop(env: Env, key: u32) {
    let mut count = 0;
    loop {
        env.storage().temporary().extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO); // Should Warn
        count += 1;
        if count >= 5 {
            break;
        }
    }
}

fn good_extend_ttl_outside_loop(env: Env, key: u32) {
    env.storage().persistent().extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO); // Good
}

#[allow(extend_ttl_in_loop, loop_invariant_storage_access)]
fn allowed_extend_ttl_in_loop(env: Env) {
    for _ in 0..10 {
        env.storage().instance().extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO); // Good (allowed)
    }
}

fn main() {}
