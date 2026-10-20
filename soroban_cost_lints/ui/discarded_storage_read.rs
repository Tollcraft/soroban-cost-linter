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

fn bad_let_underscore(env: Env) {
    let _ = env.storage().instance().get::<u32, i32>(&1); //~ WARNING storage read result is discarded
}

fn bad_statement(env: Env) {
    env.storage().persistent().get::<u32, i32>(&1); //~ WARNING storage read result is discarded
}

/// A `_`-prefixed binding is the convention for "intentionally unused" — but
/// the convention is only honest if the binding really is never read. Here it
/// is not, so the discarded read is reported.
fn bad_underscore_binding_never_read(env: Env) {
    let _cached = env.storage().instance().get::<u32, i32>(&1); //~ WARNING storage read result is discarded
}

fn good_used(env: Env) {
    let val: Option<i32> = env.storage().instance().get(&1);
    let _ = val;
}

/// A `_`-prefixed binding that is actually read later is live, not discarded.
fn good_underscore_binding_referenced(env: Env) {
    let _cached: Option<i32> = env.storage().instance().get(&1);
    if _cached.is_some() {
        // Good — the value is read below its binding
    }
}

fn good_has_branch(env: Env) {
    if env.storage().persistent().has(&1) {
        let _ = env.storage().persistent().get::<u32, i32>(&1);
    }
}

fn main() {}
