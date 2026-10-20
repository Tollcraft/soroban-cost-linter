//! Flags repeated construction of a Soroban `String` from bytes inside a loop.
//!
//! Soroban SDK 28 does not expose `String::append` or `String + String`. A
//! supported way to rebuild a growing string is `String::from_bytes`, which
//! copies and converts the byte slice into a host-backed string each time.

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{STRING_CONCAT_IN_LOOP, enclosing_loop, match_soroban_def_path};

declare_lint_pass!(StringConcatInLoop => [STRING_CONCAT_IN_LOOP]);

impl<'tcx> LateLintPass<'tcx> for StringConcatInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(callee, _args) = expr.kind
            && let ExprKind::Path(ref qpath) = callee.kind
            && let Some(def_id) = cx.qpath_res(qpath, callee.hir_id).opt_def_id()
            && match_soroban_def_path(cx, def_id, &["soroban_sdk", "String", "from_bytes"])
            && enclosing_loop(cx, expr).is_some()
        {
            span_lint_and_help(
                cx,
                STRING_CONCAT_IN_LOOP,
                expr.span,
                "repeatedly constructing a Soroban String from bytes inside a loop",
                None,
                "build the byte buffer inside the loop and construct the String once afterwards",
            );
        }
    }
}
