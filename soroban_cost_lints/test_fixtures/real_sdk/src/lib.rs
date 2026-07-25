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
// soroban_inefficient_bytes_concat — Fixtures
// =======================================================================

fn bad_bytes_push_back_in_loop(env: Env) {
    let mut bytes = soroban_sdk::Bytes::new(&env);
    for i in 0..10 {
        bytes.push_back(i as u8); // Should Warn
    }
}

fn bad_bytes_append_in_loop(env: Env) {
    let mut bytes = soroban_sdk::Bytes::new(&env);
    let other = soroban_sdk::Bytes::new(&env);
    for _ in 0..10 {
        bytes.append(&other); // Should Warn
    }
}

fn good_bytes_concat_outside_loop(env: Env) {
    let mut bytes = soroban_sdk::Bytes::new(&env);
    bytes.push_back(1); // Good — outside loop
}

fn good_vec_build_then_convert(env: Env) {
    let mut v: Vec<u8> = Vec::new();
    for i in 0..10 {
        v.push(i as u8); // Good — Vec<u8> is not Bytes
    }
    let _bytes = soroban_sdk::Bytes::from_slice(&env, &v);
}
