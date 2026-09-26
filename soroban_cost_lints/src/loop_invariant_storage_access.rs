//! Flags storage operations inside a loop whose operands are provably
//! loop-invariant.
//!
//! # Why this is a cost problem
//!
//! Soroban meters every host call. A storage read or write that does not
//! depend on the loop variable returns the same answer (or repeats the same
//! write) on every iteration, so the loop pays the full storage cost
//! `n` times to produce one result. Hoisting the operation out of the loop
//! collapses that to a single metered call.
//!
//! The companion lint `soroban_storage_in_loop` reports *any* storage
//! operation in a loop. This lint is the narrower, higher-confidence
//! subset: it only fires when it can prove the operands are invariant, so
//! the loop genuinely can be hoisted without a behavioural change.
//!
//! # How loop-invariance is decided
//!
//! [`depends_on_loop_state`] walks the loop body and the call's own
//! sub-expression (receiver chain and arguments included) and asks whether
//! any local read inside the call refers to a binding introduced by the loop
//! or to a variable the loop mutates. If it does, the call is doing real
//! per-iteration work and is not reported.
//!
//! The analysis errs towards "depends": when it cannot reach a verdict the
//! call is treated as loop-dependent and left unreported. See
//! [`depends_on_loop_state`] for the known gaps.

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Ty;
use rustc_session::declare_lint_pass;

use crate::{
    LOOP_INVARIANT_STORAGE_ACCESS, SOROBAN_STORAGE_TYPES, depends_on_loop_state, enclosing_loop,
    match_soroban_def_path, matches_any_path,
};

declare_lint_pass!(LoopInvariantStorageAccess => [LOOP_INVARIANT_STORAGE_ACCESS]);

/// Method on `soroban_sdk::Env` that hands back the [`SOROBAN_STORAGE_TYPES`]
/// accessors. A call to it is a storage access in its own right: it is the
/// first of the host calls in the `env.storage().instance().get(..)` chain.
const ENV_STORAGE_METHOD: &str = "storage";

/// Message reported for an invariant storage operation in a loop.
const LOOP_INVARIANT_STORAGE_ACCESS_MSG: &str = "loop-invariant storage operation inside a loop";

/// Help note attached to [`LOOP_INVARIANT_STORAGE_ACCESS_MSG`].
const LOOP_INVARIANT_STORAGE_ACCESS_HELP: &str = "hoist this storage operation out of the loop";

/// Late pass backing [`LOOP_INVARIANT_STORAGE_ACCESS`].
impl<'tcx> LateLintPass<'tcx> for LoopInvariantStorageAccess {
    /// Flags a storage method call inside a loop when none of its operands
    /// depend on per-iteration state (loop variables, mutated bindings).
    ///
    /// The receiver type is matched against [`SOROBAN_STORAGE_TYPES`] or
    /// recognised as `Env::storage()` by [`is_storage_access_call`].
    /// Loop-invariance is checked by [`depends_on_loop_state`]; calls that
    /// read or write loop-varying state are not reported.
    ///
    /// Note that every step in the chain is reported separately, because
    /// `check_expr` fires for each nested `MethodCall`: for
    /// `env.storage().instance().get(&1)` that is the `storage()` call, the
    /// `instance()` call and the `get()` call.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(loop_expr) = enclosing_loop(cx, expr) else {
            return;
        };

        if !is_storage_access_call(cx, expr) {
            return;
        }

        if depends_on_loop_state(cx, loop_expr, expr) {
            return;
        }

        span_lint_and_help(
            cx,
            LOOP_INVARIANT_STORAGE_ACCESS,
            expr.span,
            LOOP_INVARIANT_STORAGE_ACCESS_MSG,
            None,
            LOOP_INVARIANT_STORAGE_ACCESS_HELP,
        );
    }
}

/// Whether `expr` is a method call that reaches the host's storage subsystem.
///
/// This is the single gate for the lint, and it is deliberately kept separate
/// from the loop check so the two independent reasons for not reporting (not
/// a storage access / not loop-invariant) stay distinguishable.
///
/// Unlike `soroban_storage_in_loop`, this does **not** restrict the method
/// name: every method on a storage accessor is metered, not just the
/// terminal `get`/`has`/`set`. `Storage`, `Instance`, `Persistent` and
/// `Temporary` receivers are matched by def path, and `env.storage()` is
/// matched explicitly because `Env` is not itself in
/// [`SOROBAN_STORAGE_TYPES`].
fn is_storage_access_call<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    let ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind else {
        return false;
    };

    let method_name = path_segment.ident.name.as_str();
    let receiver_ty: Ty<'tcx> = cx.typeck_results().expr_ty(receiver).peel_refs();

    let rustc_middle::ty::Adt(adt_def, _) = receiver_ty.kind() else {
        return false;
    };
    let did = adt_def.did();

    matches_any_path(cx, did, SOROBAN_STORAGE_TYPES)
        || (match_soroban_def_path(cx, did, &["soroban_sdk", "Env"])
            && method_name == ENV_STORAGE_METHOD)
}

#[cfg(test)]
mod tests {
    use super::{ENV_STORAGE_METHOD, LOOP_INVARIANT_STORAGE_ACCESS_HELP};

    /// `storage` is matched by exact name on `soroban_sdk::Env`. This
    /// assertion documents that contract: a rename here without a
    /// corresponding change in the fixture would silently stop the `storage()`
    /// link in the chain from being reported.
    #[test]
    fn env_storage_method_name_is_stable() {
        assert_eq!(ENV_STORAGE_METHOD, "storage");
    }

    /// The help text promises a hoist. Keep it a single sentence so the note
    /// stays actionable in a terminal-width compiler output.
    #[test]
    fn help_is_a_single_sentence() {
        assert!(!LOOP_INVARIANT_STORAGE_ACCESS_HELP.contains('.'));
        assert!(LOOP_INVARIANT_STORAGE_ACCESS_HELP.contains("hoist"));
    }
}
