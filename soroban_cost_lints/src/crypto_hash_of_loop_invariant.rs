use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir as hir;
use rustc_hir::{Expr, ExprKind, QPath, def};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{
    CRYPTO_HASH_METHODS, CRYPTO_HASH_OF_LOOP_INVARIANT, depends_on_loop_state, enclosing_loop,
    is_const_expr, is_type_match,
};

declare_lint_pass!(CryptoHashOfLoopInvariant => [CRYPTO_HASH_OF_LOOP_INVARIANT]);

const CRYPTO_TYPE: &[&str] = &["soroban_sdk", "crypto", "Crypto"];

const CRYPTO_HASH_OF_LOOP_INVARIANT_MSG: &str =
    "cryptographic hash of a loop-invariant value inside a loop";

const CRYPTO_HASH_OF_LOOP_INVARIANT_HELP: &str = "compute the digest once before the loop and reuse it; the input does not change between iterations";

impl<'tcx> LateLintPass<'tcx> for CryptoHashOfLoopInvariant {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(path, receiver, args, _) = expr.kind else {
            return;
        };
        if !CRYPTO_HASH_METHODS.contains(&path.ident.name.as_str()) {
            return;
        }
        if !is_type_match(cx, cx.typeck_results().expr_ty(receiver), &[CRYPTO_TYPE]) {
            return;
        }
        let Some(loop_expr) = enclosing_loop(cx, expr) else {
            return;
        };
        let Some(arg) = args.first() else {
            return;
        };
        if is_const_expr(arg) {
            return;
        }
        let Some(local) = peeled_local(arg) else {
            return;
        };
        if declared_inside_loop(cx, local, loop_expr) || depends_on_loop_state(cx, loop_expr, expr)
        {
            return;
        }

        span_lint_and_help(
            cx,
            CRYPTO_HASH_OF_LOOP_INVARIANT,
            expr.span,
            CRYPTO_HASH_OF_LOOP_INVARIANT_MSG,
            None,
            CRYPTO_HASH_OF_LOOP_INVARIANT_HELP,
        );
    }
}

fn peeled_local(expr: &Expr<'_>) -> Option<hir::HirId> {
    let mut current = expr;
    loop {
        match current.kind {
            ExprKind::AddrOf(_, _, inner) | ExprKind::Unary(hir::UnOp::Deref, inner) => {
                current = inner;
            }
            ExprKind::Path(QPath::Resolved(None, path)) => {
                return match path.res {
                    def::Res::Local(local) => Some(local),
                    _ => None,
                };
            }
            _ => return None,
        }
    }
}

fn declared_inside_loop(cx: &LateContext<'_>, local: hir::HirId, loop_expr: &Expr<'_>) -> bool {
    let mut current = local;
    while current != loop_expr.hir_id {
        let parent = cx.tcx.parent_hir_id(current);
        if parent == current {
            return false;
        }
        current = parent;
    }
    true
}
