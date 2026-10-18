#![allow(dead_code, unused, unused_must_use, redundant_env_clone)]

pub mod soroban_sdk {
    use std::marker::PhantomData;

    pub struct Env;
    pub struct Val;
    #[derive(Debug)]
    pub struct ConversionError;

    // The four Val-boundary conversion traits. Their method def-paths
    // (`soroban_sdk::IntoVal::into_val`, ...) are what the lint matches on.
    pub trait IntoVal<E, V> {
        fn into_val(self, e: &E) -> V;
    }
    pub trait TryIntoVal<E, V> {
        fn try_into_val(self, e: &E) -> Result<V, ConversionError>;
    }
    pub trait FromVal<E, V> {
        fn from_val(e: &E, v: V) -> Self;
    }

    // Cheap native-only conversion; must NOT count as a Val hop.
    impl<E, T, U> IntoVal<E, U> for T
    where
        T: Into<U>,
    {
        fn into_val(self, _e: &E) -> U {
            self.into()
        }
    }

    // u32 <-> Val
    impl<E> IntoVal<E, Val> for u32 {
        fn into_val(self, _e: &E) -> Val {
            Val
        }
    }
    impl<E> TryIntoVal<E, u32> for Val {
        fn try_into_val(self, _e: &E) -> Result<u32, ConversionError> {
            Ok(0)
        }
    }
    impl<E> FromVal<E, Val> for u32 {
        fn from_val(_e: &E, _v: Val) -> Self {
            0
        }
    }

    // VecU32 <-> Val
    pub struct VecU32(PhantomData<u32>);
    impl<E> IntoVal<E, Val> for VecU32 {
        fn into_val(self, _e: &E) -> Val {
            Val
        }
    }
    impl<E> TryIntoVal<E, VecU32> for Val {
        fn try_into_val(self, _e: &E) -> Result<VecU32, ConversionError> {
            Ok(VecU32(PhantomData))
        }
    }
    impl<E> FromVal<E, Val> for VecU32 {
        fn from_val(_e: &E, _v: Val) -> Self {
            VecU32(PhantomData)
        }
    }
}

use soroban_sdk::{Env, FromVal, IntoVal, TryIntoVal, Val, VecU32};

// ===========================================================================
// Triggering cases — MUST produce the val_conversion_chain warning
// ===========================================================================

// Three hops carrying `base` through `Val`: u32 -> Val -> VecU32 -> Val.
fn bad_chain_three(env: &Env) {
    let base: u32 = 7;
    let v1: Val = base.into_val(env); // u32 -> Val
    let mid: VecU32 = v1.try_into_val(env).unwrap(); // Val -> VecU32
    let _f: Val = mid.into_val(env); // VecU32 -> Val
}

// Four hops mixing `into_val` and the `from_val` call form:
// u32 -> Val -> VecU32 -> Val -> u32.
fn bad_chain_four(env: &Env) {
    let base: u32 = 7;
    let v1: Val = base.into_val(env); // u32 -> Val
    let mid: VecU32 = VecU32::from_val(env, v1); // Val -> VecU32
    let v2: Val = mid.into_val(env); // VecU32 -> Val
    let _f: u32 = u32::from_val(env, v2); // Val -> u32
}

// Same as `bad_chain_three` but the first hop goes through `.clone()`, which
// must still link back to `base`.
fn bad_chain_with_clone(env: &Env) {
    let base: u32 = 7;
    let v1: Val = base.clone().into_val(env); // u32 -> Val
    let mid: VecU32 = v1.try_into_val(env).unwrap(); // Val -> VecU32
    let _f: Val = mid.into_val(env); // VecU32 -> Val
    let _also_base = base; // keeps `base` alive past the clone
}

// Four hops where the final hop is a tail expression, not a `let`.
fn bad_chain_tail(env: &Env) -> u32 {
    let base: u32 = 7;
    let v1: Val = base.into_val(env); // u32 -> Val
    let mid: VecU32 = v1.try_into_val(env).unwrap(); // Val -> VecU32
    let v2: Val = mid.into_val(env); // VecU32 -> Val
    v2.try_into_val(env).unwrap() // Val -> u32 (tail)
}

// ===========================================================================
// Non-triggering cases — MUST NOT produce a warning
// ===========================================================================

// A single conversion is just one hop; below the threshold.
fn good_single_hop(env: &Env) {
    let base: u32 = 7;
    let v: Val = base.into_val(env); // u32 -> Val
    let _ = v;
}

// Two conversions form the minimal round trip a single API boundary needs,
// and are the territory of `redundant_val_conversion`, not this lint.
fn good_two_hops(env: &Env) {
    let base: u32 = 7;
    let v: Val = base.into_val(env); // u32 -> Val
    let _back: u32 = v.try_into_val(env).unwrap(); // Val -> u32
}

// Native-only `IntoVal` (blanket `T: Into<U>`) conversions never touch `Val`,
// so they are not hops and must not chain.
fn good_native_only_chain(env: &Env) {
    let a: u32 = 1;
    let b: u64 = a.into_val(env); // u32 -> u64, no Val
    let _c: u128 = b.into_val(env); // u64 -> u128, no Val
}

// Two independent value streams do not form a single long chain.
fn good_different_values(env: &Env) {
    let base: u32 = 7;
    let v: Val = base.into_val(env); // u32 -> Val
    let other: u32 = 99;
    let v2: Val = other.into_val(env); // u32 -> Val
    let _mid: VecU32 = v2.try_into_val(env).unwrap(); // Val -> VecU32
}

fn main() {}
