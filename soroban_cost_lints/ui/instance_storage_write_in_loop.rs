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
// instance_storage_write_in_loop — Fixtures
//
// Every write below is paired with a matching `.get()` on the same
// receiver/key so that the unrelated `storage_write_without_read` lint
// stays quiet and each `.stderr` line is attributable to this lint.
// =======================================================================

// --- Should Warn: instance storage writes inside loops ---

fn bad_instance_write_in_for_loop(env: Env) {
    for i in 0..10 {
        let _existing: Option<i32> = env.storage().instance().get(&i);
        env.storage().instance().set(&i, &1); // Should Warn
    }
}

fn bad_instance_write_in_while_loop(env: Env) {
    let mut i = 0;
    while i < 10 {
        let _existing: Option<i32> = env.storage().instance().get(&i);
        env.storage().instance().set(&i, &1); // Should Warn
        i += 1;
    }
}

fn bad_instance_write_in_loop_loop(env: Env) {
    loop {
        let _existing: Option<i32> = env.storage().instance().get(&1);
        env.storage().instance().set(&1, &1); // Should Warn
        break;
    }
}

fn bad_instance_write_in_nested_loop(env: Env) {
    for i in 0..5 {
        for _ in 0..5 {
            let _existing: Option<i32> = env.storage().instance().get(&i);
            env.storage().instance().set(&i, &1); // Should Warn
        }
    }
}

fn bad_instance_write_in_for_each_closure(env: Env) {
    (0..3).for_each(|i| {
        let _existing: Option<i32> = env.storage().instance().get(&i);
        env.storage().instance().set(&i, &1); // Should Warn
    });
}

fn bad_instance_write_in_map_closure(env: Env) {
    let items = vec![1, 2, 3];
    items.iter().map(|x| {
        let _existing: Option<i32> = env.storage().instance().get(x);
        env.storage().instance().set(x, &1); // Should Warn
    }).count();
}

fn bad_instance_write_in_fold_closure(env: Env) {
    let items = vec![1, 2, 3];
    items.iter().fold(0, |acc, x| {
        let _existing: Option<i32> = env.storage().instance().get(x);
        env.storage().instance().set(x, &acc); // Should Warn
        acc + 1
    });
}

// --- Should NOT Warn: instance storage writes outside loops ---

fn good_instance_write_outside_loop(env: Env) {
    let _existing: Option<i32> = env.storage().instance().get(&1);
    env.storage().instance().set(&1, &1); // Good — not in a loop
}

fn good_instance_write_multiple_outside(env: Env) {
    let _ = env.storage().instance().get(&1);
    env.storage().instance().set(&1, &1); // Good
    let _ = env.storage().instance().get(&2);
    env.storage().instance().set(&2, &2); // Good
}

// --- Should NOT Warn: instance storage reads inside loops (reads, not writes) ---

fn good_instance_read_in_loop(env: Env) {
    for i in 0..10 {
        let _existing: Option<i32> = env.storage().instance().get(&i); // Good — read, not write
    }
}

fn good_instance_has_in_loop(env: Env) {
    for i in 0..10 {
        let _exists = env.storage().instance().has(&i); // Good — read, not write
    }
}

// --- Should NOT Warn: persistent/temporary storage writes in loops ---

fn good_persistent_write_in_loop(env: Env) {
    for i in 0..10 {
        let _existing: Option<i32> = env.storage().persistent().get(&i);
        env.storage().persistent().set(&i, &1); // Good — persistent, not instance
    }
}

fn good_temporary_write_in_loop(env: Env) {
    for i in 0..10 {
        let _existing: Option<i32> = env.storage().temporary().get(&i);
        env.storage().temporary().set(&i, &1); // Good — temporary, not instance
    }
}

// --- Suppression ---

#[allow(instance_storage_write_in_loop)]
fn allowed_instance_write_in_loop(env: Env) {
    for i in 0..10 {
        let _existing: Option<i32> = env.storage().instance().get(&i);
        env.storage().instance().set(&i, &1); // Good (allowed)
    }
}

fn main() {}
