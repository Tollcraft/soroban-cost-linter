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

// ---- blind_storage_write fixtures ----

fn bad_blind_write_instance(env: Env) {
    env.storage().instance().set(&1u32, &1i32);
}

fn bad_blind_write_persistent(env: Env) {
    env.storage().persistent().set(&2u32, &2i32);
}

fn bad_blind_write_temporary(env: Env) {
    env.storage().temporary().set(&3u32, &3i32);
}

fn good_write_after_get(env: Env) {
    let _ = env.storage().instance().get::<u32, i32>(&1u32);
    env.storage().instance().set(&1u32, &1i32);
}

fn good_write_after_has(env: Env) {
    if env.storage().persistent().has(&2u32) {
        env.storage().persistent().set(&2u32, &2i32);
    }
}