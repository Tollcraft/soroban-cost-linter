//! `excessive_vec_capacity` — flags a Soroban `Vec` pre-allocated far beyond
//! any plausible workload.
//!
//! `Vec::with_capacity(n)` and `Vec::reserve(n)` size host-side storage up
//! front: the contract is billed for the capacity it asks for, not for the
//! elements it eventually pushes. A literal capacity in the millions is almost
//! always an estimate copied from native Rust, where reserving is cheap and
//! unmetered.
//!
//! # Detection
//!
//! Only two call shapes are inspected:
//!
//! - `Vec::with_capacity(n)` — an associated-function call (`ExprKind::Call`)
//!   whose result type is one of [`SOROBAN_CONTAINER_TYPES`].
//! - `vec.reserve(n)` — a method call on a receiver of one of those types.
//!
//! The argument must be a literal integer. Runtime-valued capacities
//! (parameters, arithmetic, function results) are deliberately ignored: the
//! lint cannot know their value statically, so it stays silent rather than
//! guessing.
//!
//! A literal is reported only when it is **strictly above**
//! [`EXCESSIVE_VEC_CAPACITY_THRESHOLD`]; the exact-threshold case and anything
//! below it are silent. The threshold is the number
//! `docs/lints/excessive_vec_capacity.md` documents — keep the two in sync.

use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg};
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_hir::ExprKind;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{EXCESSIVE_VEC_CAPACITY, SOROBAN_CONTAINER_TYPES, is_type_match};

declare_lint_pass!(ExcessiveVecCapacity => [EXCESSIVE_VEC_CAPACITY]);

/// Capacity, in elements, above which a pre-allocation is reported.
///
/// This is the threshold `docs/lints/excessive_vec_capacity.md` names; the
/// fixture's `good_exact_threshold` case pins the boundary as not reported.
pub(crate) const EXCESSIVE_VEC_CAPACITY_THRESHOLD: u128 = 4096;

/// Message reported for an over-sized `with_capacity`/`reserve` call.
const EXCESSIVE_VEC_CAPACITY_MSG: &str = "excessive pre-allocation capacity in Soroban Vec";

/// Help note for `reserve`, where the receiver already exists and the call
/// cannot be rewritten to `Vec::new()` outright.
const RESERVE_HELP: &str = "consider reducing the reserve amount or using Vec::new()";

/// Suggestion offered for `Vec::with_capacity(n)`, where the whole expression
/// can be replaced by an empty vector that grows on demand.
const WITH_CAPACITY_HELP: &str = "reduce the capacity";

impl<'tcx> LateLintPass<'tcx> for ExcessiveVecCapacity {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        match expr.kind {
            // `Vec::with_capacity(n)`.
            ExprKind::Call(callee, args) => {
                let ExprKind::Path(hir::QPath::TypeRelative(_, segment)) = callee.kind else {
                    return;
                };
                if segment.ident.as_str() != "with_capacity" {
                    return;
                }
                let Some(capacity) = args.first() else {
                    return;
                };
                let Some(capacity) = literal_int(capacity) else {
                    return;
                };
                if capacity <= EXCESSIVE_VEC_CAPACITY_THRESHOLD {
                    return;
                }
                if !is_type_match(
                    cx,
                    cx.typeck_results().expr_ty(expr),
                    SOROBAN_CONTAINER_TYPES,
                ) {
                    return;
                }
                span_lint_and_sugg(
                    cx,
                    EXCESSIVE_VEC_CAPACITY,
                    expr.span,
                    EXCESSIVE_VEC_CAPACITY_MSG,
                    WITH_CAPACITY_HELP,
                    "Vec::new()".to_string(),
                    Applicability::MachineApplicable,
                );
            }
            // `vec.reserve(n)`.
            ExprKind::MethodCall(segment, receiver, args, _) => {
                if segment.ident.as_str() != "reserve" {
                    return;
                }
                let Some(capacity) = args.first() else {
                    return;
                };
                let Some(capacity) = literal_int(capacity) else {
                    return;
                };
                if capacity <= EXCESSIVE_VEC_CAPACITY_THRESHOLD {
                    return;
                }
                if !is_type_match(
                    cx,
                    cx.typeck_results().expr_ty(receiver),
                    SOROBAN_CONTAINER_TYPES,
                ) {
                    return;
                }
                span_lint_and_help(
                    cx,
                    EXCESSIVE_VEC_CAPACITY,
                    expr.span,
                    EXCESSIVE_VEC_CAPACITY_MSG,
                    None,
                    RESERVE_HELP,
                );
            }
            _ => {}
        }
    }
}

/// The value of `expr` when it is a literal integer, after peeling the
/// wrappers that do not change the value: references, parentheses, blocks
/// ending in the literal, and `as` casts.
///
/// Anything else — a parameter, a constant path, an arithmetic expression — is
/// runtime-valued as far as the lint is concerned and returns `None`.
fn literal_int(expr: &hir::Expr<'_>) -> Option<u128> {
    match expr.kind {
        ExprKind::Lit(lit) => match lit.node {
            LitKind::Int(value, _) => Some(value.get()),
            _ => None,
        },
        ExprKind::DropTemps(inner) | ExprKind::AddrOf(_, _, inner) => literal_int(inner),
        ExprKind::Cast(inner, _) => literal_int(inner),
        ExprKind::Block(block, _) => block.expr.and_then(literal_int),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{EXCESSIVE_VEC_CAPACITY_THRESHOLD, RESERVE_HELP};

    /// The docs page and the ui fixture both pin the boundary: `4096` is
    /// silent, anything above it is reported.
    #[test]
    fn threshold_matches_the_documented_boundary() {
        assert_eq!(EXCESSIVE_VEC_CAPACITY_THRESHOLD, 4096);
    }

    /// The ui golden pins this wording.
    #[test]
    fn reserve_help_is_pinned() {
        assert_eq!(
            RESERVE_HELP,
            "consider reducing the reserve amount or using Vec::new()"
        );
    }
}
