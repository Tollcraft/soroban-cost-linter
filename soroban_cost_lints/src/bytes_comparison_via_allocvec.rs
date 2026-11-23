use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{BYTES_COMPARISON_VIA_ALLOCVEC, is_type_match};

declare_lint_pass!(BytesComparisonViaAllocvec => [BYTES_COMPARISON_VIA_ALLOCVEC]);

const BYTES_TYPE: &[&[&str]] = &[&["soroban_sdk", "Bytes"]];

const MSG: &str = "comparing Bytes values through to_allocvec allocates both operands";

const HELP: &str = "compare the Bytes values directly; to_allocvec copies both host values into guest memory just to compare them";

impl<'tcx> LateLintPass<'tcx> for BytesComparisonViaAllocvec {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::Binary(op, lhs, rhs) = expr.kind else {
            return;
        };
        if !matches!(op.node, BinOpKind::Eq | BinOpKind::Ne)
            || !is_bytes_allocvec_call(cx, lhs)
            || !is_bytes_allocvec_call(cx, rhs)
        {
            return;
        }

        span_lint_and_help(
            cx,
            BYTES_COMPARISON_VIA_ALLOCVEC,
            expr.span,
            MSG,
            None,
            HELP,
        );
    }
}

fn is_bytes_allocvec_call<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    let ExprKind::MethodCall(path, receiver, [], _) = expr.kind else {
        return false;
    };
    path.ident.name.as_str() == "to_allocvec"
        && is_type_match(cx, cx.typeck_results().expr_ty(receiver), BYTES_TYPE)
}
