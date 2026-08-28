pub mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn string(&self) -> String {
            String
        }
    }

    pub struct String;
    impl String {
        pub fn from_str(_env: &Env, _s: &str) -> String {
            String
        }
        pub fn append(&self, _other: &String) -> String {
            String
        }
    }

    impl std::ops::Add for String {
        type Output = String;
        fn add(self, _rhs: String) -> String {
            String
        }
    }

    impl Clone for String {
        fn clone(&self) -> Self {
            String
        }
    }
}

use soroban_sdk::{Env, String};

// =======================================================================
// string_concat_in_loop — Fixtures
// =======================================================================

// Positive (bad): `append` on a `String` inside a `for` loop.
fn bad_string_append_in_for_loop(env: Env) {
    let mut result = String::from_str(&env, "");
    for i in 0..10 {
        let piece = String::from_str(&env, "x");
        let _ = i;
        result = result.append(&piece); // Should Warn
    }
}

// Positive (bad): `append` on a `String` inside a `while` loop.
fn bad_string_append_in_while_loop(env: Env) {
    let mut result = String::from_str(&env, "");
    let mut i = 0;
    while i < 10 {
        let piece = String::from_str(&env, "x");
        result = result.append(&piece); // Should Warn
        i += 1;
    }
}

// Positive (bad): `append` on a `String` inside an infinite `loop`.
fn bad_string_append_in_loop_loop(env: Env) {
    let mut result = String::from_str(&env, "");
    loop {
        let piece = String::from_str(&env, "x");
        result = result.append(&piece); // Should Warn
        break;
    }
}

// Positive (bad): `String + String` (`Add`) inside a `for` loop.
fn bad_string_add_in_for_loop(env: Env) {
    let mut result = String::from_str(&env, "");
    for _ in 0..10 {
        let piece = String::from_str(&env, "x");
        result = result + piece; // Should Warn
    }
}

// Positive (bad): `String + &String` (receiver is a reference) still fires.
fn bad_string_append_on_ref_in_loop(env: Env) {
    let mut result = String::from_str(&env, "");
    for _ in 0..10 {
        let piece = String::from_str(&env, "x");
        let r: &String = &result;
        let _ = r.append(&piece); // Should Warn
    }
}

// Negative (good): `append` outside any loop does not fire.
fn good_string_append_outside_loop(env: Env) {
    let result = String::from_str(&env, "");
    let piece = String::from_str(&env, "x");
    let _ = result.append(&piece); // Good — single append, not in a loop
}

// Negative (good): a single `append` inside a loop body that is not actually
// reached per-iteration because it is the loop's own accumulator pattern is
// still a loop — covered by the positive cases. Here we show a non-loop use
// of `Add` that must stay silent.
fn good_string_add_outside_loop(env: Env) {
    let a = String::from_str(&env, "a");
    let b = String::from_str(&env, "b");
    let _ = a + b; // Good — not in a loop
}

// Negative (good): `String` building via a native `Vec` buffer first, then a
// single `String` construction, does not trigger the lint.
fn good_collect_then_join(env: Env, pieces: &[&str]) {
    let mut buf: std::vec::Vec<String> = std::vec::Vec::new();
    for p in pieces {
        buf.push(String::from_str(&env, p)); // Good — accumulate in a native Vec
    }
    let _result = String::from_str(&env, "joined"); // Good — built once afterwards
}

#[allow(string_concat_in_loop)]
fn allowed_string_append_in_loop(env: Env) {
    let mut result = String::from_str(&env, "");
    for _ in 0..10 {
        let piece = String::from_str(&env, "x");
        result = result.append(&piece); // Good (allowed)
    }
}

fn main() {}
