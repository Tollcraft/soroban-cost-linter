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
            pub fn get<K: ?Sized, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K: ?Sized, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K: ?Sized>(&self, _k: &K) -> bool { false }
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K: ?Sized, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K: ?Sized, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K: ?Sized>(&self, _k: &K) -> bool { false }
        }
    }
}

use soroban_sdk::Env;

// =======================================================================
// option_wrapping_in_storage — Fixtures
// =======================================================================

// --- Triggering cases (storing Option<T> directly) ---

fn bad_option_u32(env: Env) {
    let val: Option<u32> = Some(42);
    env.storage().instance().set(&"key", &val); // Should Warn
}

fn bad_option_none(env: Env) {
    let val: Option<i64> = None;
    env.storage().persistent().set(&1, &val); // Should Warn
}

fn bad_option_from_get(env: Env) {
    let val: Option<u32> = env.storage().instance().get(&"other");
    env.storage().persistent().set(&1, &val); // Should Warn
}

fn bad_option_temporary(env: Env) {
    let val: Option<String> = Some(String::new());
    env.storage().temporary().set(&"key", &val); // Should Warn
}

// --- Non-triggering cases (storing plain T, or a struct with Option field) ---

fn good_plain_u32(env: Env) {
    env.storage().instance().set(&"key", &42u32); // Good — not Option
}

fn good_struct_with_option_field(env: Env) {
    struct Config {
        value: Option<u32>,
    }
    let cfg = Config { value: Some(10) };
    env.storage().instance().set(&"key", &cfg); // Good — struct, not Option
}

fn good_plain_string(env: Env) {
    let val = String::new();
    env.storage().persistent().set(&1, &val); // Good — String, not Option<String>
}

fn good_plain_bool(env: Env) {
    env.storage().instance().set(&"flag", &true); // Good — bool
}

#[allow(option_wrapping_in_storage)]
fn allowed_option_in_storage(env: Env) {
    let val: Option<u32> = Some(42);
    env.storage().instance().set(&"key", &val); // Good (allowed)
}

fn main() {}
