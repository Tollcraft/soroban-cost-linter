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
            pub fn try_get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn remove<K>(&self, _k: &K) -> bool { false }
            pub fn update<K>(&self, _k: &K, _f: i32) -> bool { false }
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn try_get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn remove<K>(&self, _k: &K) -> bool { false }
            pub fn update<K>(&self, _k: &K, _f: i32) -> bool { false }
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn try_get<K, V>(&self, _k: &K) -> Option<V> { None }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool { false }
            pub fn remove<K>(&self, _k: &K) -> bool { false }
            pub fn update<K>(&self, _k: &K, _f: i32) -> bool { false }
        }
    }
}

use soroban_sdk::Env;

// --- Positive: a write with no preceding read on the same key ---

/// Instance write without any read: reported.
fn normal_function_with_write_without_read(env: Env) {
    env.storage().instance().set(&1, &2); // Should Warn
}

/// Persistent write without any read: reported the same way.
fn persistent_write_without_read(env: Env) {
    env.storage().persistent().set(&1, &2); // Should Warn
}

/// Temporary write without any read: reported the same way.
fn temporary_write_without_read(env: Env) {
    env.storage().temporary().set(&1, &2); // Should Warn
}

/// A read of a *different* key does not excuse the write.
fn read_different_key(env: Env) {
    let _ = env.storage().instance().get::<i32, i32>(&1);
    env.storage().instance().set(&2, &3); // Should Warn — different key
}

/// A read on a different storage accessor is a different receiver.
fn read_on_other_accessor(env: Env) {
    let _ = env.storage().instance().get::<i32, i32>(&1);
    env.storage().persistent().set(&1, &2); // Should Warn — different receiver
}

// --- Negative: intentional initialisers are exempt by function name ---

fn initialize(env: Env) {
    env.storage().instance().set(&1, &2); // Good
}

fn set_admin(env: Env) {
    env.storage().persistent().set(&1, &2); // Good
}

// --- Negative: every observing method counts as a preceding read ---

/// `get` before `set` on the same receiver and key.
fn normal_function_with_read_and_write(env: Env) {
    let _ = env.storage().instance().get::<i32, i32>(&1);
    env.storage().instance().set(&1, &2); // Good
}

/// `has` before `set` on the same receiver and key.
fn has_before_set(env: Env) {
    let _ = env.storage().instance().has(&1);
    env.storage().instance().set(&1, &2); // Good
}

/// `try_get` before `set` on the same receiver and key.
fn try_get_before_set(env: Env) {
    let _: Option<i32> = env.storage().persistent().try_get(&1);
    env.storage().persistent().set(&1, &2); // Good
}

/// `remove` before `set` on the same receiver and key.
fn remove_before_set(env: Env) {
    let _ = env.storage().temporary().remove(&1);
    env.storage().temporary().set(&1, &2); // Good
}

/// `update` before `set` on the same receiver and key.
fn update_before_set(env: Env) {
    let _ = env.storage().instance().update(&1, 0);
    env.storage().instance().set(&1, &2); // Good
}

fn main() {}
