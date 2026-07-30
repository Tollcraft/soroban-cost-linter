// UI test suite for the `redundant_env_clone` lint rule.
//
// Each test case is self-contained with a minimal mock of `soroban_sdk::Env`
// so the file compiles without the real Soroban SDK dependency.

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
        }

        pub struct Instance;
        impl Instance {
            pub fn get<K: ?Sized, V>(&self, _k: &K) -> Option<V> {
                None
            }
            pub fn set<K: ?Sized, V>(&self, _k: &K, _v: &V) {}
        }
    }

    pub struct MyStruct {
        pub env: Env,
    }
}

use soroban_sdk::Env;

// ===========================================================================
//  Triggering cases — these MUST produce the redundant_env_clone warning
// ===========================================================================

/// Simple redundant clone: env is owned, not used after the clone.
fn bad_clone_env(env: Env) {
    let _cloned = env.clone(); //~ WARNING redundant clone on Env object
}

/// Clone result passed directly to a function; original not used after.
fn bad_clone_env_passed_to_fn(env: Env) {
    takes_env(env.clone()); //~ WARNING redundant clone on Env object
}

/// Clone inside a block expression; original not used after.
fn bad_clone_env_in_block(env: Env) {
    let _cloned = {
        env.clone() //~ WARNING redundant clone on Env object
    };
}

/// Clone result stored and original immediately shadowed (not used after).
fn bad_clone_env_shadow(env: Env) {
    let env = env.clone(); //~ WARNING redundant clone on Env object
    let _ = env;
}

// ===========================================================================
//  Non-triggering cases — these MUST NOT produce a warning
// ===========================================================================

/// Cloning a reference `&Env` produces an owned `Env`; this is legitimate.
fn good_env_ref_clone(env_ref: &Env) {
    let _cloned = env_ref.clone();
}

/// Original `env` is used after the clone on the next line.
fn good_env_used_after_clone(env: Env) {
    let _cloned = env.clone();
    let _still_here = env;
}

/// Clone is taken by a function, then original is used afterward.
fn good_fn_takes_env_by_value(env: Env) {
    let cloned = env.clone();
    takes_env(cloned);
    let _still_here = env;
}

/// No clone at all — just a reference.
fn good_no_clone(env: Env) {
    let _ref = &env;
}

/// Clone on a struct field (non-local binding) — conservatively skipped.
fn good_non_local_receiver(s: soroban_sdk::MyStruct) {
    let _cloned = s.env.clone();
}

/// Clone is explicitly allowed via attribute.
#[allow(redundant_env_clone)]
fn allowed_clone_env(env: Env) {
    let _cloned = env.clone();
}

fn takes_env(_e: Env) {}

fn main() {}
