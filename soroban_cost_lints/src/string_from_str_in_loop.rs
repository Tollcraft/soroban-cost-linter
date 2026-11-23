use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{STRING_FROM_STR_IN_LOOP, enclosing_loop, is_const_expr, match_soroban_def_path};

declare_lint_pass!(StringFromStrInLoop => [STRING_FROM_STR_IN_LOOP]);

const MSG: &str = "String::from_str with a constant value inside a loop";

const HELP: &str = "the same value is encoded and validated into a fresh host String on every iteration; build it once before the loop";

impl<'tcx> LateLintPass<'tcx> for StringFromStrInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::Call(callee, args) = expr.kind else {
            return;
        };
        let Some(def_id) = cx
            .typeck_results()
            .type_dependent_def_id(expr.hir_id)
            .or_else(|| match callee.kind {
                ExprKind::Path(qpath) => cx.qpath_res(&qpath, callee.hir_id).opt_def_id(),
                _ => None,
            })
        else {
            return;
        };
        if !match_soroban_def_path(cx, def_id, &["soroban_sdk", "String", "from_str"])
            || !matches!(args, [_, value] if is_const_expr(value))
            || enclosing_loop(cx, expr).is_none()
        {
            return;
        }

        span_lint_and_help(cx, STRING_FROM_STR_IN_LOOP, expr.span, MSG, None, HELP);
    }
}
