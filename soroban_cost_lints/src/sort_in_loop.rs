use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{SORT_IN_LOOP, enclosing_loop, is_type_match};

declare_lint_pass!(SortInLoop => [SORT_IN_LOOP]);

const SOROBAN_VEC_TYPE: &[&[&str]] = &[&["soroban_sdk", "Vec"]];
const SORT_METHODS: &[&str] = &["sort", "sort_by"];

impl<'tcx> LateLintPass<'tcx> for SortInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        let hir::ExprKind::MethodCall(path_segment, receiver, _, _) = expr.kind else {
            return;
        };
        if !SORT_METHODS.contains(&path_segment.ident.name.as_str())
            || !is_type_match(cx, cx.typeck_results().expr_ty(receiver), SOROBAN_VEC_TYPE)
            || enclosing_loop(cx, expr).is_none()
        {
            return;
        }
        span_lint_and_help(
            cx,
            SORT_IN_LOOP,
            expr.span,
            "sorting a Soroban Vec inside a loop",
            None,
            "sort once before the loop when the collection contents do not change on each iteration",
        );
    }
}
