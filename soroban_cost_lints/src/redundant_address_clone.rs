//! Flags a `clone()` on a `soroban_sdk::Address` whose original is never used
//! again.
//!
//! # Why this is a cost problem
//!
//! `Address` is a handle to a host-side object: cloning it crosses the host
//! boundary to duplicate the reference, so a clone that no later read needs is
//! pure cost. `redundant_env_clone` covers the same shape for `Env`; this lint
//! owns `Address`.
//!
//! # Conservative postures
//!
//! The lint stays silent when the clone may be genuinely needed:
//!
//! * the receiver is a `&Address` — the clone produces the owned `Address`;
//! * the original binding is read after the clone;
//! * the receiver is not a local binding (a struct field, say), where last use
//!   cannot be decided from the expression alone.

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::res::MaybeResPath;
use clippy_utils::ty::peel_and_count_ty_refs;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{
    REDUNDANT_ADDRESS_CLONE, clone_subject_is_live_after, env_clone_subject,
    match_soroban_def_path, ty_adt_def,
};

declare_lint_pass!(RedundantAddressClone => [REDUNDANT_ADDRESS_CLONE]);

/// The type this lint reports clones of.
const ADDRESS_TYPE: &[&str] = &["soroban_sdk", "Address"];

/// Message reported for a redundant `Address` clone.
const REDUNDANT_ADDRESS_CLONE_MSG: &str = "redundant clone on Address object";

/// Help note attached to [`REDUNDANT_ADDRESS_CLONE_MSG`].
const REDUNDANT_ADDRESS_CLONE_HELP: &str = "move the Address instead of cloning it";

/// Late pass backing [`REDUNDANT_ADDRESS_CLONE`].
impl<'tcx> LateLintPass<'tcx> for RedundantAddressClone {
    /// Flags `addr.clone()` when `addr` is an owned `Address` local that is not
    /// read afterwards.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(subject) = env_clone_subject(cx, expr) else {
            return;
        };

        let subject_ty = cx.typeck_results().expr_ty(subject);
        let Some(adt_def) = ty_adt_def(subject_ty.peel_refs()) else {
            return;
        };
        if !match_soroban_def_path(cx, adt_def.did(), ADDRESS_TYPE) {
            return;
        }

        // Cloning a `&Address` produces the owned `Address` the caller asked
        // for; there is nothing redundant about it.
        let (_inner, ref_count, _) = peel_and_count_ty_refs(subject_ty);
        if ref_count > 0 {
            return;
        }

        // Only a local binding has a decidable last use. A field access or any
        // other non-local receiver is skipped.
        if subject.res_local_id().is_none() {
            return;
        }

        if clone_subject_is_live_after(cx, subject, expr) {
            return;
        }

        span_lint_and_help(
            cx,
            REDUNDANT_ADDRESS_CLONE,
            expr.span,
            REDUNDANT_ADDRESS_CLONE_MSG,
            None,
            REDUNDANT_ADDRESS_CLONE_HELP,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{ADDRESS_TYPE, REDUNDANT_ADDRESS_CLONE_HELP};

    #[test]
    fn only_address_is_matched() {
        assert_eq!(ADDRESS_TYPE, &["soroban_sdk", "Address"]);
    }

    #[test]
    fn help_suggests_moving() {
        assert!(REDUNDANT_ADDRESS_CLONE_HELP.contains("move"));
    }
}
