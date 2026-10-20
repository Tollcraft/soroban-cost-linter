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
// soroban_storage_in_loop — Fixtures
// =======================================================================

fn bad_storage_in_for_loop(env: Env) {
    for i in 0..10 {
        env.storage().instance().set(&i, &1); // Should Warn
    }
}

fn bad_storage_in_while_loop(env: Env) {
    let mut i = 0;
    while i < 10 {
        let _: Option<i32> = env.storage().persistent().get(&i); // Should Warn
        i += 1;
    }
}

fn bad_storage_in_loop_loop(env: Env) {
    loop {
        if env.storage().temporary().has(&1) { // Should Warn
            break;
        }
    }
}

fn bad_storage_in_nested_loop(env: Env) {
    for i in 0..5 {
        for _ in 0..5 {
            env.storage().instance().has(&i); // Should Warn
        }
    }
}

fn good_storage_outside_loop(env: Env) {
    env.storage().instance().set(&1, &1); // Good
}

fn good_storage_multiple_outside(env: Env) {
    let _ = env.storage().temporary().has(&1); // Good
    let _ = env.storage().persistent().get::<i32, i32>(&1); // Good
    env.storage().instance().set(&2, &2); // Good
}

fn bad_storage_in_for_each_closure(env: Env) {
    (0..3).for_each(|i| {
        env.storage().instance().set(&i, &1); // Should Warn
    });
}

fn bad_storage_in_map_closure(env: Env) {
    let items = vec![1, 2, 3];
    items.iter().map(|x| {
        env.storage().instance().set(x, &1); // Should Warn
    }).count();
}

fn bad_storage_in_fold_closure(env: Env) {
    let items = vec![1, 2, 3];
    items.iter().fold(0, |acc, x| {
        env.storage().instance().set(x, &acc); // Should Warn
        acc + 1
    });
}

fn good_storage_in_option_map(env: Env) {
    let opt = Some(42);
    opt.map(|x| {
        env.storage().instance().set(&x, &1); // Good — Option::map calls at most once
    });
}

#[allow(soroban_storage_in_loop)]
fn allowed_storage_in_loop(env: Env) {
    for i in 0..10 {
        env.storage().instance().set(&i, &1); // Good (allowed)
    }
}

// ---------------------------------------------------------------------------
// The read-modifying accessors: `update`, `try_update` and `remove` each pay
// a ledger read plus a ledger write per call, so they must lint exactly like
// `set` inside a loop and stay quiet outside one.
// ---------------------------------------------------------------------------

fn bad_update_in_for_loop(env: Env) {
    for i in 0..10 {
        env.storage().persistent().update(&i, |v: i32| v + 1); // Should Warn
    }
}

fn bad_update_instance_in_for_loop(env: Env) {
    for i in 0..10 {
        env.storage().instance().update(&i, |v: i32| v + 1); // Should Warn
    }
}

fn bad_try_update_in_for_loop(env: Env) {
    for i in 0..10 {
        let _ = env.storage().persistent().try_update(&i, |v: i32| v); // Should Warn
    }
}

fn bad_try_update_instance_in_for_loop(env: Env) {
    for i in 0..10 {
        let _ = env.storage().instance().try_update(&i, |v: i32| v); // Should Warn
    }
}

fn bad_remove_in_for_loop(env: Env) {
    for i in 0..10 {
        let _: Option<i32> = env.storage().persistent().remove(&i); // Should Warn
    }
}

fn bad_remove_instance_in_for_loop(env: Env) {
    for i in 0..10 {
        let _: Option<i32> = env.storage().instance().remove(&i); // Should Warn
    }
}

fn good_rw_methods_outside_loop(env: Env) {
    env.storage().persistent().update(&1, |v: i32| v + 1); // Good
    let _ = env.storage().instance().try_update(&1, |v: i32| v); // Good
    let _: Option<i32> = env.storage().persistent().remove(&1); // Good
}

fn main() {}

// The read-modifying accessors under test are declared after `main` so every
// pre-existing diagnostic keeps its blessed line number in the `.stderr`.
impl soroban_sdk::storage::Instance {
    pub fn update<F, K, V>(&self, _k: &K, _f: F) -> V
    where
        F: FnOnce(V) -> V,
    {
        unimplemented!()
    }

    pub fn try_update<F, K, V>(&self, _k: &K, _f: F) -> Result<V, ()>
    where
        F: FnOnce(V) -> V,
    {
        unimplemented!()
    }

    pub fn remove<K, V>(&self, _k: &K) -> Option<V> {
        unimplemented!()
    }
}

impl soroban_sdk::storage::Persistent {
    pub fn update<F, K, V>(&self, _k: &K, _f: F) -> V
    where
        F: FnOnce(V) -> V,
    {
        unimplemented!()
    }

    pub fn try_update<F, K, V>(&self, _k: &K, _f: F) -> Result<V, ()>
    where
        F: FnOnce(V) -> V,
    {
        unimplemented!()
    }

    pub fn remove<K, V>(&self, _k: &K) -> Option<V> {
        unimplemented!()
    }
}
