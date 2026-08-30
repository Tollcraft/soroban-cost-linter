#![allow(
    soroban_storage_in_loop,
    loop_invariant_storage_access,
    storage_write_without_read,
    persistent_read_without_ttl_extension,
    instance_storage_for_unbounded_data,
    unbounded_input_loop,
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
// nested_loop_storage_access — Fixtures
// =======================================================================

// ── Depth 2: should fire ──────────────────────────────────────────────

fn nested_for_for(env: Env) {
    for i in 0..5 {
        for j in 0..5 {
            env.storage().instance().set(&(i + j), &1); // Should Warn (depth 2)
        }
    }
}

fn nested_for_while(env: Env) {
    for i in 0..5 {
        let mut j = 0;
        while j < 5 {
            let _: Option<i32> = env.storage().persistent().get(&i); // Should Warn (depth 2)
            j += 1;
        }
    }
}

fn nested_while_for(env: Env) {
    let mut i = 0;
    while i < 5 {
        for j in 0..5 {
            env.storage().instance().has(&(i + j)); // Should Warn (depth 2)
        }
        i += 1;
    }
}

fn nested_loop_loop(env: Env) {
    loop {
        loop {
            if env.storage().temporary().has(&1) { // Should Warn (depth 2)
                break;
            }
            break;
        }
        break;
    }
}

// ── Closure inside loop (depth 1 only — should NOT fire) ──────────────

fn closure_inside_for(env: Env) {
    let items = vec![1, 2, 3];
    items.iter().for_each(|x| {
        env.storage().instance().set(x, &1); // Good — depth 1 (closure doesn't add nesting)
    });
}

fn nested_loop_with_closure(env: Env) {
    for i in 0..5 {
        let items = vec![1, 2, 3];
        items.iter().for_each(|x| {
            env.storage().instance().set(&(i + x), &1); // Good — depth 1 (for_each closure is not a loop)
        });
    }
}

// ── Depth 1: should NOT fire ─────────────────────────────────────────

fn single_for_loop(env: Env) {
    for i in 0..10 {
        env.storage().instance().set(&i, &1); // Good — depth 1
    }
}

fn single_while_loop(env: Env) {
    let mut i = 0;
    while i < 10 {
        let _: Option<i32> = env.storage().persistent().get(&i); // Good — depth 1
        i += 1;
    }
}

fn single_loop_loop(env: Env) {
    loop {
        if env.storage().temporary().has(&1) { // Good — depth 1
            break;
        }
    }
}

// ── Function definition inside loop (depth should reset) ──────────────

fn nested_function_not_counted(env: Env) {
    for i in 0..5 {
        fn inner_func(env: &Env, x: i32) {
            for j in 0..5 {
                env.storage().instance().set(&(x + j), &1); // Good — inner_func's loops are not nested inside outer
            }
        }
        inner_func(&env, i);
    }
}

// ── No storage operation: should NOT fire ─────────────────────────────

fn nested_loops_no_storage(env: Env) {
    for i in 0..5 {
        for j in 0..5 {
            let _ = i + j; // Good — no storage operation
        }
    }
}

// ── Storage outside all loops: should NOT fire ────────────────────────

fn storage_outside_loops(env: Env) {
    for i in 0..5 {
        for j in 0..5 {
            let _ = i + j;
        }
    }
    env.storage().instance().set(&1, &1); // Good — outside all loops
}

#[allow(nested_loop_storage_access)]
fn allowed_nested(env: Env) {
    for i in 0..5 {
        for j in 0..5 {
            env.storage().instance().set(&(i + j), &1); // Good (allowed)
        }
    }
}

fn main() {}
