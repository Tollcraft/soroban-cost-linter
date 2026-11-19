//! Flags a `Val`-boundary conversion that produces nothing new.
//!
//! # Why this is a cost problem
//!
//! Crossing the Soroban native/`Val` boundary is a metered host call plus a
//! (de)serialisation. Two shapes waste that call outright:
//!
//! 1. **Same-type conversion** — `let _: u32 = num.into_val(&env);` converts a
//!    value into the type it already has. The call is a no-op: the representation
//!    never changes, so the crossing buys nothing.
//! 2. **Round trip** — `u32::try_from_val(&env, &num.into_val(&env))` pushes a
//!    value to `Val` and straight back to its original type inside one
//!    expression. Two boundary crossings are paid to hand back the value that
//!    was put in.
//!
//! Converting a value *once*, directly to the representation the next step
//! needs, moves the same data in a single crossing.
//!
//! # Detection
//!
//! The pass inspects every expression (`check_expr`) and matches the conversion
//! traits by definition path (`soroban_sdk::IntoVal::into_val`, its siblings)
//! rather than by method name alone, so the mock module in the ui fixture and
//! the real SDK are handled identically. Two guards keep it honest:
//!
//! * Concrete types only. A conversion is redundant only when both sides are
//!   resolved, *non-generic* types that are equal; an inference variable or a
//!   type parameter (`fn f<T: IntoVal<Env, T>>(t: T) -> T`) lines up only
//!   incidentally, so it is left alone rather than reported.
//! * One and two hops only. Chains of three or more `Val` crossings belong to
//!   the sibling [`crate::VAL_CONVERSION_CHAIN`] lint, which is documented as
//!   owning them; deferring those here keeps the two passes disjoint.
//!
//! The round trip is matched only in its *nested* form — the `from_val`/
//! `try_from_val` argument is itself the `into_val` call. A two-hop round trip
//! spread across separate bindings is a chain a data-flow pass should own, not
//! a syntactic no-op, and is left to the sibling.

use clippy_utils::diagnostics::span_lint_and_help;

use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Ty;
use rustc_session::declare_lint_pass;
use rustc_span::sym;

use crate::{REDUNDANT_VAL_CONVERSION, match_soroban_def_path};

declare_lint_pass!(RedundantValConversion => [REDUNDANT_VAL_CONVERSION]);

/// Def-path of `soroban_sdk::IntoVal::into_val`.
const INTO_VAL: &[&str] = &["soroban_sdk", "IntoVal", "into_val"];

/// Def-path of `soroban_sdk::ToVal::to_val`.
const TO_VAL: &[&str] = &["soroban_sdk", "ToVal", "to_val"];

/// Def-path of `soroban_sdk::FromVal::from_val`, called as `T::from_val(env, val)`.
const FROM_VAL: &[&str] = &["soroban_sdk", "FromVal", "from_val"];

/// Def-path of `soroban_sdk::TryFromVal::try_from_val`, called as
/// `T::try_from_val(env, val)`.
const TRY_FROM_VAL: &[&str] = &["soroban_sdk", "TryFromVal", "try_from_val"];

impl<'tcx> LateLintPass<'tcx> for RedundantValConversion {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let Some(message) = redundant_conversion(cx, expr) {
            span_lint_and_help(
                cx,
                REDUNDANT_VAL_CONVERSION,
                expr.span,
                message.0,
                None,
                message.1,
            );
        }
    }
}

/// Whether `expr` is a redundant conversion, and the message/help to emit.
///
/// Returns `None` for anything that is a genuine single crossing into a new
/// representation, a generic or unresolved conversion, or a chain long enough
/// to belong to [`crate::VAL_CONVERSION_CHAIN`].
fn redundant_conversion<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'static str, &'static str)> {
    if let Some(source) = into_val_source(cx, expr) {
        // Case (a): `value.into_val(env)` / `value.to_val(env)` landing on the
        // type the value already has.
        let target = cx.typeck_results().expr_ty(expr);
        if concrete_and_equal(source, target) {
            return Some((
                SAME_TYPE_MSG,
                "remove this conversion since the value is already the target type",
            ));
        }
    }

    // Case (b): `T::from_val(env, arg)` / `T::try_from_val(env, arg)` whose
    // `arg` is, through a borrow, an `into_val` of a value already of type `T`
    // — a `T -> Val -> T` round trip in a single expression.
    if let ExprKind::Call(callee, args) = expr.kind
        && let ExprKind::Path(ref qpath) = callee.kind
        && let QPath::TypeRelative(_, segment) = qpath
        && let Some(path) = match segment.ident.as_str() {
            "from_val" => Some(FROM_VAL),
            "try_from_val" => Some(TRY_FROM_VAL),
            _ => None,
        }
        && let Some(did) = cx.qpath_res(qpath, callee.hir_id).opt_def_id()
        && match_soroban_def_path(cx, did, path)
        && let Some(arg) = args.get(1)
        && let Some(inner) = into_val_source(cx, strip_borrows(arg))
    {
        let target = ok_ty(cx, cx.typeck_results().expr_ty(expr));
        if concrete_and_equal(inner, target) {
            return Some((
                ROUND_TRIP_MSG,
                "remove these conversions and use the original value directly",
            ));
        }
    }

    None
}

/// The method name of an `into_val`/`to_val` boundary call, matched by
/// definition path, and the type of the value it converts.
///
/// `None` means `expr` is not such a call, so callers can treat the
/// conversion's own receiver type uniformly for both the same-type and
/// round-trip cases.
fn into_val_source<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<Ty<'tcx>> {
    if let ExprKind::MethodCall(segment, receiver, _, _) = expr.kind
        && let Some(path) = match segment.ident.as_str() {
            "into_val" => Some(INTO_VAL),
            "to_val" => Some(TO_VAL),
            _ => None,
        }
        && let Some(did) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
        && match_soroban_def_path(cx, did, path)
    {
        return Some(cx.typeck_results().expr_ty(receiver).peel_refs());
    }
    None
}

/// Peel the borrow prefixes off a call argument (`&num.into_val(env)` ->
/// the `num.into_val(env)` expression) so an argument reference does not hide
/// the conversion inside it.
fn strip_borrows<'tcx>(expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    let mut expr = expr;
    while let ExprKind::AddrOf(_, _, inner) = expr.kind {
        expr = inner;
    }
    expr
}

/// Whether `ty` is a resolved, non-generic type (not an inference variable and
/// not a type parameter). Only concrete types make a conversion provably
/// redundant.
fn is_concrete(ty: Ty<'_>) -> bool {
    !matches!(
        ty.kind(),
        rustc_middle::ty::Infer(_) | rustc_middle::ty::Param(_)
    )
}

/// Whether both sides are concrete types that are the same once references are
/// stripped.
fn concrete_and_equal<'tcx>(source: Ty<'tcx>, target: Ty<'tcx>) -> bool {
    let source = source.peel_refs();
    let target = target.peel_refs();
    is_concrete(source) && is_concrete(target) && source == target
}

/// The `T` out of a `Result<T, E>`, so a `try_from_val` round trip is compared
/// by the value it yields rather than by the `Result` wrapper.
fn ok_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
    if let Some(result_did) = cx.tcx.get_diagnostic_item(sym::Result)
        && let rustc_middle::ty::Adt(def, substs) = ty.kind()
        && def.did() == result_did
        && let Some(ok) = substs[0].as_type()
    {
        return ok;
    }
    ty
}

/// The same-type warning text; both strings are pinned by the ui golden.
const SAME_TYPE_MSG: &str = "redundant conversion to the same type";

/// The round-trip warning text; pinned by the ui golden.
const ROUND_TRIP_MSG: &str = "redundant round-trip conversion across the Val boundary";

#[cfg(test)]
mod tests {
    use super::{FROM_VAL, INTO_VAL, ROUND_TRIP_MSG, SAME_TYPE_MSG, TO_VAL, TRY_FROM_VAL};

    /// Exactly the four boundary conversions the pass reasons about, matched by
    /// definition path. A typo here is a silent false negative on every real
    /// contract.
    #[test]
    fn the_conversion_paths_are_the_boundary_conversions() {
        assert_eq!(INTO_VAL, ["soroban_sdk", "IntoVal", "into_val"]);
        assert_eq!(TO_VAL, ["soroban_sdk", "ToVal", "to_val"]);
        assert_eq!(FROM_VAL, ["soroban_sdk", "FromVal", "from_val"]);
        assert_eq!(TRY_FROM_VAL, ["soroban_sdk", "TryFromVal", "try_from_val"]);
    }

    /// The four paths must be pairwise distinct: two traits sharing a method
    /// name tail must not be conflated.
    #[test]
    fn the_conversion_paths_are_pairwise_distinct() {
        let all = [INTO_VAL, TO_VAL, FROM_VAL, TRY_FROM_VAL];
        for (index, path) in all.iter().enumerate() {
            for other in &all[index + 1..] {
                assert_ne!(path, other, "conversion paths must be distinct");
            }
        }
    }

    /// The ui golden pins this wording verbatim.
    #[test]
    fn messages_match_the_golden() {
        assert_eq!(SAME_TYPE_MSG, "redundant conversion to the same type");
        assert_eq!(
            ROUND_TRIP_MSG,
            "redundant round-trip conversion across the Val boundary"
        );
    }
}
