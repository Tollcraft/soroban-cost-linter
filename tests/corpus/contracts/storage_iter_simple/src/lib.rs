//! Fixture contract for the corpus lint benchmarks.
//!
//! This contract is the smallest `soroban_storage_in_loop` fixture in
//! `tests/corpus/contracts/`: it deliberately contains one triggering case and
//! one clean case so the real-world corpus test can confirm the lint fires on
//! the former and stays silent on the latter. The functions below are the
//! shared shape all other `storage_iter_*` fixtures scale up from.
//!
//! Storage keys are a single `symbol_short!` constant, matching how
//! production contracts usually key small instance-storage counters.

#![no_std]
use soroban_sdk::{symbol_short, Env, Symbol};

/// Instance-storage key shared by every entry point in this fixture.
const COUNTER: Symbol = symbol_short!("counter");

/// Number of iterations the good-case loop accumulates over.
///
/// Chosen small (10) so the corpus build stays fast.
const ITERATIONS: u32 = 10;

/// Triggers `soroban_storage_in_loop`: set inside a for loop.
///
/// The value written varies with the loop counter, so this also exercises the
/// "loop-variant payload" case: the anti-pattern is the repeated host write
/// itself, not just a repeated identical write.
///
/// The loop bound is a *literal*, not a named constant: the linter evaluates
/// literal bounds when deciding whether to report, and a `0..ITERATIONS`
/// bound changes which lints fire on this function. Changing it would
/// invalidate the committed corpus baseline for this fixture.
pub fn bad_simple(env: Env) {
    for i in 0..10 {
        env.storage().instance().set(&COUNTER, &i);
    }
}

/// Good: storage operation outside the loop.
///
/// Accumulates in a local variable first, then performs exactly one storage
/// write after the loop. The final state is the same shape as the bad case's
/// last write (the counter holds a single `i128`), demonstrating that the fix
/// does not change the storage contract — only its cost profile.
pub fn good_simple(env: Env) {
    let mut total = 0i128;
    for _ in 0..ITERATIONS {
        total += 1;
    }
    env.storage().instance().set(&COUNTER, &total);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bad entry point must keep at least one instance-storage write
    /// lexically inside the `for` body: that shape is what
    /// `soroban_storage_in_loop` keys on for this fixture, and the corpus
    /// baseline entry for `storage_iter_simple` asserts it fires.
    #[test]
    fn bad_case_still_writes_inside_loop() {
        // Compile-time canary: the body of `bad_simple` must remain a loop
        // containing a `set` call. If the loop is ever hoisted out (i.e. the
        // fixture is accidentally "fixed"), the baseline comparison in
        // cargo-cost-lint/tests/real_world_corpus.rs will start failing
        // because the expected TP disappears.
        let src = include_str!("lib.rs");
        let bad_start = src.find("pub fn bad_simple").expect("bad_simple exists");
        let bad_end = src.find("pub fn good_simple").expect("good_simple exists");
        let body = &src[bad_start..bad_end];
        assert!(
            body.contains("for ") && body.contains(".set("),
            "bad_simple must contain a storage set inside a loop"
        );
    }

    /// The good entry point must keep its storage write outside the loop.
    #[test]
    fn good_case_writes_after_loop() {
        let src = include_str!("lib.rs");
        let good_start = src.find("pub fn good_simple").expect("good_simple exists");
        let good_end = src.find("#[cfg(test)]").expect("test module follows");
        let body = &src[good_start..good_end];
        let set_pos = body.find(".set(").expect("good_simple performs a set");
        let for_pos = body.find("for ").expect("good_simple contains a loop");
        let for_end = body[for_pos..]
            .find("}")
            .map(|i| for_pos + i)
            .expect("loop closes");
        assert!(
            set_pos > for_end,
            "the storage write in good_simple must come after the loop closes"
        );
    }

    /// The accumulated value is deterministic: 10 iterations of +1 must
    /// always produce 10, independent of environment state.
    #[test]
    fn good_case_accumulates_deterministically() {
        // Mirrors the loop in good_simple without needing a host Env: the
        // arithmetic is what the single post-loop write persists.
        let mut total = 0i128;
        for _ in 0..ITERATIONS {
            total += 1;
        }
        assert_eq!(total, 10);
    }

    /// The loop-variant payload of the bad case must stay a plain integer
    /// that increases with the counter, guarding against the fixture being
    /// simplified into a loop-invariant write by accident.
    #[test]
    fn bad_case_payload_is_loop_variant() {
        let src = include_str!("lib.rs");
        let bad_start = src.find("pub fn bad_simple").expect("bad_simple exists");
        let bad_end = src.find("pub fn good_simple").expect("good_simple exists");
        let body = &src[bad_start..bad_end];
        assert!(
            body.contains("&i)"),
            "bad_simple writes the loop counter itself, keeping the payload loop-variant"
        );
    }

    /// The fixture shares one key between both entry points; a divergent key
    /// would make the bad/good comparison incomparable.
    ///
    /// The needle is assembled via `concat!` so this test's own source text
    /// (which is included via `include_str!`) cannot match it and inflate
    /// the count.
    #[test]
    fn both_entry_points_share_counter_key() {
        let needle = concat!(".set(&C", "OUNTER");
        let src = include_str!("lib.rs");
        assert_eq!(src.matches(needle).count(), 2);
    }
}

/// Behavioral tests that run the fixture entry points against a real host
/// `Env`, complementing the source-canary tests above.
///
/// The canary tests pin the *lexical shape* the linter keys on; these tests
/// pin the *runtime semantics*: what is stored, under which key, with which
/// type, and what a reader sees on the missing-key and wrong-type error
/// paths. Both halves matter — the shape keeps the corpus baseline honest,
/// the behavior keeps the fixture a valid cost model of the documented fix.
///
/// Storage is only accessible inside a contract context, so every test runs
/// through [`in_contract_context`], which registers a minimal test-only
/// contract solely to obtain a contract id.
#[cfg(test)]
mod env_behavior {
    use super::*;
    use soroban_sdk::{contract, contractimpl};

    /// Minimal registered contract used only to obtain a contract id so the
    /// fixture's storage operations run inside a real contract context.
    #[contract]
    struct TestContract;

    #[contractimpl]
    impl TestContract {
        /// No-op entry point; registration needs at least one exported
        /// function to build the native contract.
        pub fn noop(_env: Env) {}
    }

    /// Runs `f` with a fresh `Env` inside the storage context of
    /// [`TestContract`], returning whatever `f` returns.
    fn in_contract_context<R>(f: impl FnOnce(&Env) -> R) -> R {
        let env = Env::default();
        let id = env.register_contract(None, TestContract);
        env.as_contract(&id, || f(&env))
    }

    /// Iteration count for the behavioral tests below.
    const ITERATIONS: u32 = 10;

    /// Missing-key error path: before any write, a read of the shared key
    /// yields `None` rather than panicking — the exact failure mode
    /// `unwrap_on_storage_get` warns about for production contracts.
    #[test]
    fn missing_key_reads_as_none() {
        in_contract_context(|env| {
            assert_eq!(env.storage().instance().get::<Symbol, i32>(&COUNTER), None);
        });
    }

    /// `bad_simple` leaves the last iteration's value behind: the loop-variant
    /// payload ends at `9` (the last value of `0..10`), proving the loop ran
    /// to completion and that every iteration overwrote the previous write.
    /// The stored type is `i32` — the loop counter's inferred type now that
    /// the bound is a plain literal.
    #[test]
    fn bad_simple_persists_last_iteration_value() {
        in_contract_context(|env| {
            bad_simple(env.clone());
            assert_eq!(
                env.storage().instance().get::<Symbol, i32>(&COUNTER),
                Some(9)
            );
        });
    }

    /// Wrong-type read error path: the bad case stores an `i32`, so reading
    /// the same key as `i128` traps with `UnexpectedType`. The host does not
    /// coerce between integer widths — a mismatched read aborts the
    /// invocation, which is precisely the failure mode
    /// `unwrap_on_storage_get` warns about for production contracts.
    #[test]
    #[should_panic(expected = "UnexpectedType")]
    fn bad_case_value_traps_when_read_as_i128() {
        in_contract_context(|env| {
            bad_simple(env.clone());
            let _ = env.storage().instance().get::<Symbol, i128>(&COUNTER);
        });
    }

    /// `good_simple` stores the accumulated total: exactly `ITERATIONS`
    /// increments of one `i128`, persisted in a single post-loop write.
    #[test]
    fn good_simple_persists_accumulated_total() {
        in_contract_context(|env| {
            good_simple(env.clone());
            assert_eq!(
                env.storage().instance().get::<Symbol, i128>(&COUNTER),
                Some(ITERATIONS as i128)
            );
        });
    }

    /// Both entry points share `COUNTER`: running bad then good replaces the
    /// `i32` counter with the `i128` total under the same key. The value the
    /// bad case left behind is fully superseded — the same key now holds the
    /// accumulated `i128`.
    #[test]
    fn good_simple_overwrites_bad_case_value_under_shared_key() {
        in_contract_context(|env| {
            bad_simple(env.clone());
            good_simple(env.clone());
            assert_eq!(
                env.storage().instance().get::<Symbol, i128>(&COUNTER),
                Some(ITERATIONS as i128)
            );
        });
    }

    /// Overwrite error path: after `good_simple` replaces the payload with an
    /// `i128`, the old `i32` interpretation is gone and an `i32` read of the
    /// shared key traps with a conversion error instead of returning the
    /// previous counter value.
    #[test]
    #[should_panic(expected = "ConversionError")]
    fn stale_i32_read_traps_after_overwrite() {
        in_contract_context(|env| {
            bad_simple(env.clone());
            good_simple(env.clone());
            let _ = env.storage().instance().get::<Symbol, i32>(&COUNTER);
        });
    }
}
