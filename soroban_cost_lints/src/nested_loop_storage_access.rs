//! Flags storage access reached two or more loops deep.
//!
//! # Why this is a cost problem
//!
//! A storage operation inside a single loop already costs O(n) host calls.
//! Nested one level deeper it costs O(n·m) — the product of both trip counts —
//! which is qualitatively worse than the linear case and almost never worth
//! the ledger traffic. This lint is declared `Deny` for that reason: the
//! diagnostic is an error, not a warning.
//!
//! # How depth is counted
//!
//! A pass over each function body keeps a counter that is incremented for
//! `for`/`while`/`loop` expressions. Two scopes deliberately do **not**
//! change the counter:
//!
//! * a closure body — the enclosing loop determines how often it runs, so
//!   `for_each` inside a single loop stays at depth one;
//! * a nested `fn` definition — its body has its own counter, since the outer
//!   loop only decides how often the function is *called*, and the function
//!   is analysed on its own `check_fn` visit.
//!
//! Only the terminal storage methods are matched, so the `storage()` and
//! `instance()` steps of a chain are not reported separately: the whole chain
//! is reported once, at its outermost call.

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir as hir;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{NESTED_LOOP_STORAGE_ACCESS, SOROBAN_STORAGE_TYPES, matches_any_path, ty_adt_def};

declare_lint_pass!(NestedLoopStorageAccess => [NESTED_LOOP_STORAGE_ACCESS]);

/// Loop nesting depth at which a storage access is reported.
const MIN_REPORTED_DEPTH: usize = 2;

/// Terminal storage methods: reads, writes, and TTL extension. Accessor steps
/// such as `storage()` and `instance()` are excluded so a chain is reported
/// once, at its outermost call.
const STORAGE_METHODS: &[&str] = &[
    "get",
    "try_get",
    "has",
    "set",
    "put",
    "update",
    "try_update",
    "remove",
    "extend_ttl",
    "extend_ttl_with_limits",
];

/// Message reported for a storage access nested two loops deep.
const NESTED_LOOP_STORAGE_ACCESS_MSG: &str =
    "storage operation inside a nested loop — cost is O(n·m)";

/// Help note attached to [`NESTED_LOOP_STORAGE_ACCESS_MSG`].
const NESTED_LOOP_STORAGE_ACCESS_HELP: &str = "this storage access runs on every iteration of both loops, so its cost is multiplicative rather than linear; hoist it out of at least one loop or accumulate mutations in memory";

/// Late pass backing [`NESTED_LOOP_STORAGE_ACCESS`].
impl<'tcx> LateLintPass<'tcx> for NestedLoopStorageAccess {
    /// Walks each function body once, tracking loop nesting depth.
    ///
    /// Starting a fresh visitor per body is what makes a nested `fn` reset the
    /// counter: its body is a separate `check_fn` call, and the visitor below
    /// does not descend into nested items.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: intravisit::FnKind<'tcx>,
        _: &'tcx hir::FnDecl<'tcx>,
        body: &'tcx hir::Body<'tcx>,
        _: rustc_span::Span,
        _: hir::def_id::LocalDefId,
    ) {
        let mut visitor = DepthVisitor { cx, depth: 0 };
        visitor.visit_body(body);
    }
}

/// Counts enclosing loops and reports terminal storage calls at depth ≥ 2.
struct DepthVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    depth: usize,
}

impl<'tcx> Visitor<'tcx> for DepthVisitor<'_, 'tcx> {
    /// Increments the depth for a loop body and reports storage accesses that
    /// are already [`MIN_REPORTED_DEPTH`] or more loops deep.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Loop(..) = expr.kind {
            self.depth += 1;
            intravisit::walk_expr(self, expr);
            self.depth -= 1;
            return;
        }

        if self.depth >= MIN_REPORTED_DEPTH && is_storage_access(self.cx, expr) {
            span_lint_and_help(
                self.cx,
                NESTED_LOOP_STORAGE_ACCESS,
                expr.span,
                NESTED_LOOP_STORAGE_ACCESS_MSG,
                None,
                NESTED_LOOP_STORAGE_ACCESS_HELP,
            );
        }

        intravisit::walk_expr(self, expr);
    }

    /// A nested item (a `fn` defined inside a block) starts a new body with its
    /// own loop counter, so it is not visited here. `check_fn` covers it.
    fn visit_item(&mut self, _: &'tcx hir::Item<'tcx>) {}
}

/// Whether `expr` is a terminal storage method call on one of the Soroban
/// storage accessor types.
fn is_storage_access<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    let ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind else {
        return false;
    };
    if !STORAGE_METHODS.contains(&path_segment.ident.name.as_str()) {
        return false;
    }
    ty_adt_def(cx.typeck_results().expr_ty(receiver).peel_refs())
        .is_some_and(|adt_def| matches_any_path(cx, adt_def.did(), SOROBAN_STORAGE_TYPES))
}

#[cfg(test)]
mod tests {
    use super::{MIN_REPORTED_DEPTH, STORAGE_METHODS};

    #[test]
    fn depth_two_is_the_threshold() {
        assert_eq!(MIN_REPORTED_DEPTH, 2);
    }

    #[test]
    fn accessor_steps_are_not_terminal() {
        assert!(!STORAGE_METHODS.contains(&"storage"));
        assert!(!STORAGE_METHODS.contains(&"instance"));
        assert!(!STORAGE_METHODS.contains(&"persistent"));
        assert!(!STORAGE_METHODS.contains(&"temporary"));
    }
}
