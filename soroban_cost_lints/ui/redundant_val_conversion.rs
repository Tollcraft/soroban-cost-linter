#![allow(unused)]
#![warn(redundant_val_conversion)]

pub mod soroban_sdk {
    pub struct Env;
    pub struct Val;

    pub trait IntoVal<E, V> {
        fn into_val(&self, env: &E) -> V;
    }

    pub trait TryFromVal<E, V> {
        type Error;
        fn try_from_val(env: &E, v: &V) -> Result<Self, Self::Error>
        where
            Self: Sized;
    }

    impl IntoVal<Env, Val> for u32 {
        fn into_val(&self, _env: &Env) -> Val {
            Val
        }
    }

    impl TryFromVal<Env, Val> for u32 {
        type Error = ();
        fn try_from_val(_env: &Env, _v: &Val) -> Result<Self, Self::Error> {
            Ok(0)
        }
    }

    impl IntoVal<Env, u32> for u32 {
        fn into_val(&self, _env: &Env) -> u32 {
            *self
        }
    }
}

use soroban_sdk::{Env, Val, IntoVal, TryFromVal};

// Case 1: converting a value into the type it already is.
fn flag_same_type(env: Env, num: u32) {
    let _: u32 = num.into_val(&env); //~ WARNING redundant conversion to the same type
}

// Case 2: a round trip — converted into Val and immediately back to its
// original type within the same expression chain.
fn flag_round_trip(env: Env, num: u32) {
    let _: Result<u32, ()> = u32::try_from_val(&env, &num.into_val(&env)); //~ WARNING redundant round-trip conversion
}

// Good: u32 -> Val is a genuine boundary crossing.
fn good_valid_conversion(env: Env, num: u32) {
    let _val: Val = num.into_val(&env);
}

// Good: explicitly suppressed.
#[allow(redundant_val_conversion)]
fn allowed_round_trip(env: Env, num: u32) {
    let _: Result<u32, ()> = u32::try_from_val(&env, &num.into_val(&env));
}

fn main() {}
