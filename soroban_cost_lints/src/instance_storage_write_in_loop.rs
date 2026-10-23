//! Flags `set`/`put`/`update` on the Soroban *instance* storage tier when the
//! call sits inside a loop or inside a repeatedly-called closure.
//!
//! # Why this is a cost problem
//!
//! Instance storage travels with the contract instance: every invocation loads
//! it, and every write to it is paid again by whoever reads the instance next.
//! Writing it once per iteration multiplies the ledger write cost by the
//! iteration count while the value that survives the loop is usually the last
//! one written.
//!
//! # Relationship to `soroban_storage_in_loop`
//!
//! `soroban_storage_in_loop` denies storage operations in loops generally.
//! This lint is the tier-specific subset for instance writes, and it also
//! covers writes inside a closure the loop drives (`for_each`, `map`,
//! `fold`), which the general lint reaches through its loop-or-closure check.
//! Because the two overlap, a fixture that wants each diagnostic attributable
//! to this lint allows the general one in its header. When both are enabled
//! `soroban_storage_in_loop` is registered first and therefore reports first.

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::get_enclosing_loop_or_multi_call_closure;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{INSTANCE_STORAGE_WRITE_IN_LOOP, match_soroban_def_path, ty_adt_def};

declare_lint_pass!(InstanceStorageWriteInLoop => [INSTANCE_STORAGE_WRITE_IN_LOOP]);

/// The only storage tier this lint reports; `Persistent` and `Temporary`
/// writes keep their own lints.
const INSTANCE_STORAGE_TYPE: &[&str] = &["soroban_sdk", "storage", "Instance"];

/// Mutating methods on a storage accessor. Reads (`get`, `has`) and
/// `remove` are deliberately absent: only a write pays the instance-entry
/// ledger write.
const WRITE_METHODS: &[&str] = &["set", "put", "update"];

/// Message reported for an instance write in a loop.
const INSTANCE_STORAGE_WRITE_IN_LOOP_MSG: &str = "instance storage written inside a loop";

/// Help note attached to [`INSTANCE_STORAGE_WRITE_IN_LOOP_MSG`].
const INSTANCE_STORAGE_WRITE_IN_LOOP_HELP: &str =
    "accumulate the value in memory and write the instance entry once after the loop";

/// Late pass backing [`INSTANCE_STORAGE_WRITE_IN_LOOP`].
impl<'tcx> LateLintPass<'tcx> for InstanceStorageWriteInLoop {
    /// Flags a write on an `Instance` receiver whose enclosing scope is a loop
    /// or a closure that is called more than once.
    ///
    /// The loop-or-closure test is [`get_enclosing_loop_or_multi_call_closure`],
    /// so `for_each`/`map`/`fold` bodies are covered without treating a
    /// one-shot closure as a loop.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind else {
            return;
        };

        if !WRITE_METHODS.contains(&path_segment.ident.name.as_str()) {
            return;
        }

        let Some(adt_def) = ty_adt_def(cx.typeck_results().expr_ty(receiver).peel_refs()) else {
            return;
        };
        if !match_soroban_def_path(cx, adt_def.did(), INSTANCE_STORAGE_TYPE) {
            return;
        }

        if get_enclosing_loop_or_multi_call_closure(cx, expr).is_none() {
            return;
        }

        span_lint_and_help(
            cx,
            INSTANCE_STORAGE_WRITE_IN_LOOP,
            expr.span,
            INSTANCE_STORAGE_WRITE_IN_LOOP_MSG,
            None,
            INSTANCE_STORAGE_WRITE_IN_LOOP_HELP,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{INSTANCE_STORAGE_TYPE, WRITE_METHODS};

    #[test]
    fn only_instance_tier_is_matched() {
        assert_eq!(
            INSTANCE_STORAGE_TYPE,
            &["soroban_sdk", "storage", "Instance"]
        );
    }

    #[test]
    fn reads_are_not_writes() {
        for method in ["get", "try_get", "has"] {
            assert!(!WRITE_METHODS.contains(&method), "{method} is a read");
        }
        for method in ["set", "put", "update"] {
            assert!(WRITE_METHODS.contains(&method), "{method} is a write");
        }
    }
}
