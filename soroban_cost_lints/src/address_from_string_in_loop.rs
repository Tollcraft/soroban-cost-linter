use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::res::MaybeDef;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{ADDRESS_FROM_STRING_IN_LOOP, enclosing_loop, match_soroban_def_path};

declare_lint_pass!(AddressFromStringInLoop => [ADDRESS_FROM_STRING_IN_LOOP]);

impl<'tcx> LateLintPass<'tcx> for AddressFromStringInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        let hir::ExprKind::Call(callee, _) = expr.kind else {
            return;
        };
        let hir::ExprKind::Path(ref qpath) = callee.kind else {
            return;
        };
        let Some(def_id) = cx.qpath_res(qpath, callee.hir_id).opt_def_id() else {
            return;
        };
        if !match_soroban_def_path(cx, def_id, &["soroban_sdk", "Address", "from_string"])
            || enclosing_loop(cx, expr).is_none()
        {
            return;
        }
        span_lint_and_help(
            cx,
            ADDRESS_FROM_STRING_IN_LOOP,
            expr.span,
            "Address::from_string called inside a loop",
            None,
            "parse the address once outside the loop, the strkey is loop-invariant",
        );
    }
}
