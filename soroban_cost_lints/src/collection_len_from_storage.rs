use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::get_parent_expr;
use clippy_utils::ty::option_arg_ty;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{
    COLLECTION_LEN_FROM_STORAGE, SOROBAN_CONTAINER_TYPES, SOROBAN_STORAGE_TYPES, is_type_match,
};

declare_lint_pass!(CollectionLenFromStorage => [COLLECTION_LEN_FROM_STORAGE]);

const COLLECTION_LEN_FROM_STORAGE_MSG: &str =
    "stored collection loaded from storage only to call len()";

const COLLECTION_LEN_FROM_STORAGE_HELP: &str =
    "use `has` on the key or keep a stored counter instead of decoding the whole collection";

impl<'tcx> LateLintPass<'tcx> for CollectionLenFromStorage {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(path, receiver, args, _) = expr.kind else {
            return;
        };
        if path.ident.name.as_str() != "get" || args.len() != 1 {
            return;
        }
        if !is_type_match(
            cx,
            cx.typeck_results().expr_ty(receiver),
            SOROBAN_STORAGE_TYPES,
        ) {
            return;
        }

        let Some(inner_ty) = option_arg_ty(cx, cx.typeck_results().expr_ty(expr)) else {
            return;
        };
        if !is_type_match(cx, inner_ty, SOROBAN_CONTAINER_TYPES) {
            return;
        }

        let Some(unwrapped) = method_parent(cx, expr, &["unwrap", "expect"]) else {
            return;
        };
        if method_parent(cx, unwrapped, &["len", "is_empty"]).is_none() {
            return;
        }

        span_lint_and_help(
            cx,
            COLLECTION_LEN_FROM_STORAGE,
            args[0].span,
            COLLECTION_LEN_FROM_STORAGE_MSG,
            None,
            COLLECTION_LEN_FROM_STORAGE_HELP,
        );
    }
}

fn method_parent<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    methods: &[&str],
) -> Option<&'tcx Expr<'tcx>> {
    let parent = get_parent_expr(cx, expr)?;
    if let ExprKind::MethodCall(path, receiver, _, _) = parent.kind
        && methods.contains(&path.ident.name.as_str())
        && receiver.hir_id == expr.hir_id
    {
        return Some(parent);
    }
    None
}
