use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::LitKind;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{
    SOROBAN_STORAGE_TYPES, STRING_TYPE_AS_STORAGE_KEY, is_type_match, is_valid_short_symbol,
    match_soroban_def_path,
};

declare_lint_pass!(StringTypeAsStorageKey => [STRING_TYPE_AS_STORAGE_KEY]);

const KEY_METHODS: &[&str] = &["get", "has", "set"];

const STRING_TYPE_AS_STORAGE_KEY_MSG: &str =
    "String storage key built from a short literal where a Symbol would do";

const STRING_TYPE_AS_STORAGE_KEY_HELP: &str = "use `symbol_short!` or a fixed-width integer key: the key is encoded, hashed and stored on every access";

impl<'tcx> LateLintPass<'tcx> for StringTypeAsStorageKey {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(path, receiver, args, _) = expr.kind else {
            return;
        };
        if !KEY_METHODS.contains(&path.ident.name.as_str()) || args.is_empty() {
            return;
        }
        if !is_type_match(
            cx,
            cx.typeck_results().expr_ty(receiver),
            SOROBAN_STORAGE_TYPES,
        ) {
            return;
        }

        let key = peel_ref_expr(&args[0]);
        let ExprKind::Call(callee, call_args) = key.kind else {
            return;
        };
        let ExprKind::Path(qpath) = callee.kind else {
            return;
        };
        let Some(callee_id) = cx.qpath_res(&qpath, callee.hir_id).opt_def_id() else {
            return;
        };
        if !match_soroban_def_path(cx, callee_id, &["soroban_sdk", "String", "from_str"])
            || call_args.len() != 2
        {
            return;
        }
        let ExprKind::Lit(lit) = call_args[1].kind else {
            return;
        };
        let LitKind::Str(symbol, _) = lit.node else {
            return;
        };
        if !is_valid_short_symbol(symbol.as_str()) {
            return;
        }

        span_lint_and_help(
            cx,
            STRING_TYPE_AS_STORAGE_KEY,
            args[0].span,
            STRING_TYPE_AS_STORAGE_KEY_MSG,
            None,
            STRING_TYPE_AS_STORAGE_KEY_HELP,
        );
    }
}

fn peel_ref_expr<'a, 'tcx>(expr: &'a Expr<'tcx>) -> &'a Expr<'tcx> {
    let mut current = expr;
    loop {
        match current.kind {
            ExprKind::AddrOf(_, _, inner) | ExprKind::Unary(_, inner) => current = inner,
            _ => return current,
        }
    }
}
