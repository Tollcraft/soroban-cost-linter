#![no_std]
use soroban_sdk::Env;

fn bad_storage_in_loop(env: Env) {
    for _ in 0..10 {
        env.storage().instance().set(&1u32, &1i32);
    }
}

fn bad_storage_in_while_loop(env: Env) {
    let mut i = 0;
    while i < 10 {
        let _ = env.storage().persistent().get::<u32, i32>(&i);
        i += 1;
    }
}

fn bad_storage_in_loop_loop(env: Env) {
    loop {
        if env.storage().temporary().has(&1u32) {
            break;
        }
    }
}

fn good_storage_outside_loop(env: Env) {
    env.storage().instance().set(&1u32, &1i32);
}

fn bad_clone_env(env: Env) {
    let _cloned = env.clone();
}

fn bad_host_call_in_loop(env: Env) {
    for _ in 0..10 {
        let _seq = env.ledger().sequence();
    }
}

fn bad_host_in_loop(env: Env) {
    /*
    for _ in 0..10 {
        let _host = env.host().clone();
        let _ = host.budget_cloned();
    }
    */
}

fn good_host_outside_loop(env: Env) {
    /*
    let host = env.host().clone();
    let _ = host.budget_cloned();
    for _ in 0..10 {
        // do not use host inside loop
    }
    */
}

// =======================================================================
// soroban_redundant_storage_read — Fixtures
// =======================================================================

fn bad_sequential_get_same_key(env: Env) {
    let key = 1u32;
    let _a: Option<i32> = env.storage().instance().get(&key);
    let _b: Option<i32> = env.storage().instance().get(&key); // Should Warn
}

fn bad_sequential_has_then_get(env: Env) {
    let key = 1u32;
    let _a = env.storage().instance().has(&key);
    let _b: Option<i32> = env.storage().instance().get(&key); // Should Warn
}

fn bad_sequential_has_then_has(env: Env) {
    let key = 1u32;
    let _a = env.storage().instance().has(&key);
    let _b = env.storage().instance().has(&key); // Should Warn
}

fn good_different_keys(env: Env) {
    let key1 = 1u32;
    let key2 = 2u32;
    let _a: Option<i32> = env.storage().instance().get(&key1);
    let _b: Option<i32> = env.storage().instance().get(&key2); // Good — different key
}

fn good_write_resets(env: Env) {
    let key = 1u32;
    let _a: Option<i32> = env.storage().instance().get(&key);
    env.storage().instance().set(&key, &1i32);
    let _b: Option<i32> = env.storage().instance().get(&key); // Good — write in between
}
