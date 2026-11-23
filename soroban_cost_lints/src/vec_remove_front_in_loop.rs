use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::LitKind;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{VEC_REMOVE_FRONT_IN_LOOP, enclosing_loop, is_type_match};

declare_lint_pass!(VecRemoveFrontInLoop => [VEC_REMOVE_FRONT_IN_LOOP]);

const VEC_TYPE: &[&[&str]] = &[&["soroban_sdk", "Vec"]];

const MSG: &str = "Vec::remove(0) inside a loop";

const HELP: &str = "removing from the front shifts every remaining element on each iteration; drain into a native Vec and rebuild the Soroban Vec once";

impl<'tcx> LateLintPass<'tcx> for VecRemoveFrontInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(path, receiver, args, _) = expr.kind else {
            return;
        };
        if path.ident.name.as_str() != "remove"
            || !matches!(args, [index] if is_zero_literal(index))
            || !is_type_match(cx, cx.typeck_results().expr_ty(receiver), VEC_TYPE)
            || enclosing_loop(cx, expr).is_none()
        {
            return;
        }

        span_lint_and_help(cx, VEC_REMOVE_FRONT_IN_LOOP, expr.span, MSG, None, HELP);
    }
}

fn is_zero_literal(expr: &Expr<'_>) -> bool {
    matches!(expr.kind, ExprKind::Lit(lit) if matches!(lit.node, LitKind::Int(value, _) if value.get() == 0))
}
