pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env {
        fn clone(&self) -> Self {
            Env
        }
    }
    impl Env {
        pub fn storage(&self) -> storage::Storage {
            storage::Storage
        }
    }

    pub mod storage {
        pub struct Storage;
        impl Storage {
            pub fn instance(&self) -> Instance {
                Instance
            }
            pub fn persistent(&self) -> Persistent {
                Persistent
            }
            pub fn temporary(&self) -> Temporary {
                Temporary
            }
        }

        pub struct Instance;
        impl Instance {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> {
                None
            }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool {
                false
            }
        }

        pub struct Persistent;
        impl Persistent {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> {
                None
            }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool {
                false
            }
        }

        pub struct Temporary;
        impl Temporary {
            pub fn get<K, V>(&self, _k: &K) -> Option<V> {
                None
            }
            pub fn set<K, V>(&self, _k: &K, _v: &V) {}
            pub fn has<K>(&self, _k: &K) -> bool {
                false
            }
        }
    }
}

use soroban_sdk::Env;

// Should Warn: writing back unchanged value
fn bad_write_unchanged(env: Env) {
    let val: i32 = env.storage().instance().get(&1).unwrap();
    env.storage().instance().set(&1, &val);
}

// Should Not Warn: value is mutated
fn good_write_mutated(env: Env) {
    let mut val: i32 = env.storage().instance().get(&1).unwrap();
    val += 1;
    env.storage().instance().set(&1, &val);
}

// Should Not Warn: write is guarded by inequality check
fn good_write_guarded(env: Env, new_val: i32) {
    let val: i32 = env.storage().instance().get(&1).unwrap();
    if val != new_val {
        env.storage().instance().set(&1, &new_val);
    }
}

// Should Not Warn: different key
fn good_write_different_key(env: Env) {
    let val: i32 = env.storage().instance().get(&1).unwrap();
    env.storage().instance().set(&2, &val);
}

// Should Not Warn: allowed
#[allow(storage_write_of_unchanged_value)]
fn allow_suppresses(env: Env) {
    let val: i32 = env.storage().instance().get(&1).unwrap();
    env.storage().instance().set(&1, &val);
}
