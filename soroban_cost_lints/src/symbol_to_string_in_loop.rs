use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{
    SYMBOL_TO_STRING_IN_LOOP, cached_def_path_str, enclosing_loop, is_type_match,
    match_soroban_def_path,
};

declare_lint_pass!(SymbolToStringInLoop => [SYMBOL_TO_STRING_IN_LOOP]);

const SYMBOL_TYPE: &[&str] = &["soroban_sdk", "Symbol"];

const SYMBOL_TO_STRING_IN_LOOP_MSG: &str = "Symbol::to_string called inside a loop";

const SYMBOL_TO_STRING_IN_LOOP_HELP: &str =
    "compare the `Symbol` values directly, or build the `String` once before the loop";

impl<'tcx> LateLintPass<'tcx> for SymbolToStringInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(path, receiver, _, _) = expr.kind else {
            return;
        };
        if path.ident.name.as_str() != "to_string" {
            return;
        }
        if !is_type_match(cx, cx.typeck_results().expr_ty(receiver), &[SYMBOL_TYPE]) {
            return;
        }
        if !resolves_to_to_string(cx, expr) || enclosing_loop(cx, expr).is_none() {
            return;
        }

        span_lint_and_help(
            cx,
            SYMBOL_TO_STRING_IN_LOOP,
            expr.span,
            SYMBOL_TO_STRING_IN_LOOP_MSG,
            None,
            SYMBOL_TO_STRING_IN_LOOP_HELP,
        );
    }
}

fn resolves_to_to_string(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return true;
    };
    if match_soroban_def_path(cx, def_id, &["soroban_sdk", "Symbol", "to_string"]) {
        return true;
    }
    let path = cached_def_path_str(cx.tcx, def_id);
    path == "to_string" || path.ends_with("::to_string")
}
