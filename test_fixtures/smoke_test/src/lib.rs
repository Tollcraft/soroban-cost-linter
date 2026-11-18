//! Fixture contract for the smoke test exercised by
//! `.github/workflows/container-publish.yml`.
//!
//! The container workflow mounts this directory standalone, runs
//! `cargo cost-lint --format json` inside it, and asserts that the
//! `soroban_storage_in_loop` finding appears in the JSON output. The empty
//! `[workspace]` table in `Cargo.toml` keeps cargo from treating the fixture as
//! part of the root workspace, which would make the container smoke test fail
//! with a workspace-membership error before the linter ever runs.
//!
//! # Layout
//!
//! Every entry point exposes the same anti-pattern — a storage write performed
//! inside a loop — so the container test only needs one lint name to appear,
//! regardless of which entry point the compiler happens to reach first:
//!
//! * [`bad_storage_in_loop`] — the original minimal case from the container
//!   workflow. Kept verbatim so the workflow's assertion stays meaningful.
//! * [`bad_storage_in_loop_nested`] — the same write guarded by an `if`, to
//!   show the pattern survives control flow inside the loop body.
//! * [`bad_storage_in_loop_multiple_keys`] — two distinct writes per iteration,
//!   which also fires `loop_invariant_storage_access` on the invariant write.
//!
//! Refactoring guidance: keep this module no_std, keep each helper free of
//! unused parameters, and keep at least one `instance().set()` inside a loop.

#![no_std]
use soroban_sdk::Env;

/// Writes a constant to instance storage once per iteration of a `for` loop.
///
/// This is the case the container smoke test greps for: `soroban_storage_in_loop`
/// must appear in the linter's JSON output for this contract.
///
/// # Why it is expensive
///
/// Each iteration issues a full host storage write for a value that never
/// changes, spending CPU instructions and ledger write throughput on redundant
/// work. A single write after the loop produces the same final state at a
/// fraction of the cost.
pub fn bad_storage_in_loop(env: Env) {
    for _ in 0..10 {
        env.storage().instance().set(&1u32, &1i32);
    }
}

/// Writes inside a loop with conditional control flow.
///
/// Complements [`bad_storage_in_loop`] by showing that the anti-pattern is not
/// limited to a bare loop body: a nested `if` inside the iteration still leaves
/// every executed iteration performing the same storage write.
pub fn bad_storage_in_loop_nested(env: Env) {
    for i in 0..10 {
        if i < 5 {
            env.storage().instance().set(&2u32, &i);
        }
    }
}

/// Performs two storage writes per iteration, one of them loop-invariant.
///
/// The `&3u32` write is identical in every iteration, so it also exercises
/// `loop_invariant_storage_access` alongside `soroban_storage_in_loop`. The
/// `&4u32` write varies with the loop counter, isolating the loop-detection
/// behaviour from the invariance analysis.
pub fn bad_storage_in_loop_multiple_keys(env: Env) {
    for i in 0..10 {
        env.storage().instance().set(&3u32, &1i32);
        env.storage().instance().set(&4u32, &i);
    }
}

/// Demonstrates suppression of `soroban_storage_in_loop` via attribute.
///
/// Used to verify whether source-level `#[allow]` attributes take precedence
/// over `budget.toml` configuration pinning the lint to `deny`.
#[allow(soroban_storage_in_loop)]
pub fn allowed_storage_in_loop(env: Env) {
    for _ in 0..10 {
        env.storage().instance().set(&5u32, &1i32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Guards the smoke-test contract: the three helpers above must keep the
    /// `Env` parameter as the first argument so the container workflow's single
    /// `cargo cost-lint` invocation keeps working if the helpers are renamed or
    /// reordered. Renaming a helper does not affect the workflow because it
    /// asserts on the lint name, not on function names.
    #[test]
    fn helpers_take_env_first() {
        fn check(_f: fn(Env)) {
            // Existence is the assertion: this only compiles when every
            // helper keeps the `fn(Env)` shape.
        }
        check(bad_storage_in_loop);
        check(bad_storage_in_loop_nested);
        check(bad_storage_in_loop_multiple_keys);
    }
}
