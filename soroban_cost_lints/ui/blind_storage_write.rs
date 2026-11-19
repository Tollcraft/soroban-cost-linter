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
}

use soroban_sdk::Env;

// =======================================================================
// blind_storage_write — Fixtures
// =======================================================================

// Firing: a read of the key exists somewhere in the function, then the key is
// written twice. The second write discards the first store without reading it
// back, so it is a blind overwrite. `storage_write_without_read` stays silent
// here because the key IS read.
fn blind_overwrite_after_read(env: Env) {
    let _ = env.storage().instance().get::<i32, i32>(&1);
    env.storage().instance().set(&1, &2); // first write — informed, no warn
    env.storage().instance().set(&1, &3); // Should Warn: blind overwrite
}

// Boundary with `storage_write_without_read`: the key is never read, so that
// lint fires on BOTH writes, while `blind_storage_write` stays silent (no read
// anywhere in the function).
fn blind_not_fired_when_no_read(env: Env) {
    env.storage().instance().set(&1, &2); // storage_write_without_read warns
    env.storage().instance().set(&1, &3); // storage_write_without_read warns
}

// Near-miss: the second write is informed by a fresh read between the two
// writes, so it is not blind. Neither lint fires.
fn informed_overwrite(env: Env) {
    let _ = env.storage().instance().get::<i32, i32>(&1);
    env.storage().instance().set(&1, &2); // first write — informed
    let _ = env.storage().instance().get::<i32, i32>(&1); // read since last write
    env.storage().instance().set(&1, &3); // informed — no warn
}

// Initialising a brand-new key with a single write is never flagged by
// `blind_storage_write` (there is no prior write to discard).
fn init_new_key(env: Env) {
    env.storage().instance().set(&1, &2); // Good
}

// Allowed: suppression works.
#[allow(blind_storage_write)]
fn allowed_blind_overwrite(env: Env) {
    let _ = env.storage().instance().get::<i32, i32>(&1);
    env.storage().instance().set(&1, &2);
    env.storage().instance().set(&1, &3); // would warn, but allowed
}

fn main() {}
