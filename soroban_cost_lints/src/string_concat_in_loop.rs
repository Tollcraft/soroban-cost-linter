//! Flags repeated concatenation of a `soroban_sdk::String` inside a loop.
//!
//! # Why this is a cost problem
//!
//! A host-side `String` is immutable. `append` does not grow an in-place
//! buffer: it allocates a fresh host buffer and copies both operands into it,
//! returning a new value. Building a string of `n` pieces inside a loop with
//! `result = result.append(&piece)` therefore performs
//! `1 + 2 + ... + n` byte copies — quadratic in the number of iterations, and
//! quadratic in the metered CPU/memory budget because every intermediate
//! buffer is a real host allocation.
//!
//! The fix is to stop concatenating per iteration: collect the pieces in a
//! native collection (`Vec<String>`, or byte slices) and build the `String`
//! once after the loop.
//!
//! # Detection
//!
//! Two syntactic shapes are recognised, because both perform the same
//! host-side copy:
//!
//! 1. `receiver.append(&other)` where the receiver resolves to
//!    `soroban_sdk::String` — see [`is_append_call_on_string`].
//! 2. `a + b` where *either* operand resolves to `soroban_sdk::String` —
//!    `String` implements `Add`, and the desugared operator call is not a
//!    method call, so it needs its own check — see [`is_string_addition`].
//!
//! Both are reported only when the expression sits inside a syntactic loop.
//! As with `bytes_append_in_loop`, the lint deliberately does **not** try to
//! decide whether the loop *could* be batched: that reasoning is
//! runtime-dependent and would inflate the false-positive rate. A single
//! concatenation, even in a loop, may well be exactly what the author wants.

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Ty;
use rustc_session::declare_lint_pass;

use crate::{STRING_CONCAT_IN_LOOP, enclosing_loop, match_soroban_def_path, ty_adt_def};

declare_lint_pass!(StringConcatInLoop => [STRING_CONCAT_IN_LOOP]);

/// Methods on `soroban_sdk::String` that concatenate, allocating a fresh
/// buffer and copying on every call.
///
/// Only `append` is listed. Soroban exposes no in-place `push_str`-style
/// method on `String`, so any future growth method would have to be added
/// here explicitly rather than picked up by a prefix match.
const STRING_CONCAT_METHODS: &[&str] = &["append"];

/// Def path suffix identifying the host-side `String` wrapper.
const SOROBAN_STRING_PATH: &[&str] = &["soroban_sdk", "String"];

/// Message reported for a concatenation on a `soroban_sdk::String` in a loop.
const STRING_CONCAT_IN_LOOP_MSG: &str = "repeatedly concatenating a soroban String inside a loop";

/// Help note attached to [`STRING_CONCAT_IN_LOOP_MSG`].
const STRING_CONCAT_IN_LOOP_HELP: &str = "collect the pieces in a native collection (e.g. `Vec<String>` or byte \
     slices) inside the loop and construct the `String` a single time \
     afterwards; pre-size where practical";

/// Late pass backing [`STRING_CONCAT_IN_LOOP`].
impl<'tcx> LateLintPass<'tcx> for StringConcatInLoop {
    /// Flags a concatenation on a `soroban_sdk::String` inside a loop.
    ///
    /// Matching is done two ways — an `append` method call and a `+`
    /// (binary `Add`) — see the [module documentation](self). Only syntactic
    /// loops are considered; multi-call closures are not flagged here.
    ///
    /// The `append` branch returns early after reporting, so an expression
    /// that somehow satisfies both shapes is reported once.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if is_append_call_on_string(cx, expr) && enclosing_loop(cx, expr).is_some() {
            span_lint_and_help(
                cx,
                STRING_CONCAT_IN_LOOP,
                expr.span,
                STRING_CONCAT_IN_LOOP_MSG,
                None,
                STRING_CONCAT_IN_LOOP_HELP,
            );
            return;
        }

        if is_string_addition(cx, expr) && enclosing_loop(cx, expr).is_some() {
            span_lint_and_help(
                cx,
                STRING_CONCAT_IN_LOOP,
                expr.span,
                STRING_CONCAT_IN_LOOP_MSG,
                None,
                STRING_CONCAT_IN_LOOP_HELP,
            );
        }
    }
}

/// Whether `expr` is a `<soroban_sdk::String>::append(..)` method call.
///
/// The method name is checked first because it is the cheapest test and
/// rejects the overwhelming majority of method calls in a crate before any
/// type lookup happens. Only then is the receiver's type resolved, which is
/// the comparatively expensive part.
fn is_append_call_on_string<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    if let ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind
        && STRING_CONCAT_METHODS.contains(&path_segment.ident.name.as_str())
    {
        return is_string_type(cx, cx.typeck_results().expr_ty(receiver));
    }
    false
}

/// Whether `expr` is a `+` expression with a `soroban_sdk::String` on either
/// side.
///
/// Either operand is accepted rather than only the left one: `&String + String`
/// and `String + &String` both reach the same `Add` implementation after
/// auto-deref, and both copy the same bytes.
fn is_string_addition<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    if let ExprKind::Binary(op, lhs, rhs) = &expr.kind
        && matches!(op.node, BinOpKind::Add)
    {
        let results = cx.typeck_results();
        return is_string_type(cx, results.expr_ty(lhs))
            || is_string_type(cx, results.expr_ty(rhs));
    }
    false
}

/// Whether `ty` resolves to `soroban_sdk::String`.
///
/// References are peeled first so that a `String` reached through `&result`
/// is matched the same way as an owned one. Non-ADT types (`&str`, integers,
/// ranges, …) fall out here without a def-path lookup.
fn is_string_type(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    ty_adt_def(ty.peel_refs())
        .is_some_and(|adt_def| match_soroban_def_path(cx, adt_def.did(), SOROBAN_STRING_PATH))
}

#[cfg(test)]
mod tests {
    use super::{SOROBAN_STRING_PATH, STRING_CONCAT_METHODS};

    /// `append` is the only concatenation method the SDK exposes on `String`;
    /// widening this list would flag non-concatenating calls too.
    #[test]
    fn concat_method_list_is_exactly_append() {
        assert_eq!(STRING_CONCAT_METHODS, ["append"]);
    }

    /// The def-path suffix is matched with `str::ends_with`, so the final
    /// component must stay the bare type name — a trailing module path or a
    /// generic argument list would stop matching real `String` receivers.
    #[test]
    fn string_def_path_suffix_is_bare_type_name() {
        assert_eq!(SOROBAN_STRING_PATH, ["soroban_sdk", "String"]);
        let joined = SOROBAN_STRING_PATH.join("::");
        assert!(joined.ends_with("String"));
    }
}
