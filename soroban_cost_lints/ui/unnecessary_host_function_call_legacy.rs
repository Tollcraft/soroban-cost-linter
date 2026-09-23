#![allow(soroban_storage_in_loop, storage_write_without_read, redundant_env_clone)]

pub mod soroban_sdk {
    pub struct Env;
    impl Clone for Env {
        fn clone(&self) -> Self { Env }
    }
    impl Env {
        pub fn host(&self) -> Host {
            Host
        }
    }

    pub struct Host;
    impl Host {
        pub fn ecdsa_recover(&self) -> bool { true }
    }
}

use soroban_sdk::Env;

// =======================================================================
// unnecessary_host_function_call_legacy — Fixtures
// =======================================================================

// --- Positive (should warn): direct host access instead of the SDK
//     convenience wrapper ---

fn bad_host_call_legacy(env: Env) {
    let _ok = env.host().ecdsa_recover();
}

fn bad_host_call_legacy_in_loop(env: Env) {
    for _ in 0..10 {
        let _ok = env.host().ecdsa_recover();
    }
}

// --- Negative (should not warn): ordinary SDK-path usage ---

fn good_sdk_wrapper(env: Env) {
    let _ = env; // real lints route through a high-level call
}

fn main() {}