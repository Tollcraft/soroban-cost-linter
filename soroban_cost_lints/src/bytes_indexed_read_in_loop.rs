use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{BYTES_INDEXED_READ_IN_LOOP, enclosing_loop, is_type_match};

declare_lint_pass!(BytesIndexedReadInLoop => [BYTES_INDEXED_READ_IN_LOOP]);

const SOROBAN_BYTES_TYPE: &[&[&str]] = &[&["soroban_sdk", "Bytes"]];

impl<'tcx> LateLintPass<'tcx> for BytesIndexedReadInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        let hir::ExprKind::MethodCall(path_segment, receiver, _, _) = expr.kind else {
            return;
        };
        if path_segment.ident.name.as_str() != "get"
            || !is_type_match(
                cx,
                cx.typeck_results().expr_ty(receiver),
                SOROBAN_BYTES_TYPE,
            )
            || enclosing_loop(cx, expr).is_none()
        {
            return;
        }
        span_lint_and_help(
            cx,
            BYTES_INDEXED_READ_IN_LOOP,
            expr.span,
            "per-byte Bytes::get call inside a loop",
            None,
            "copy the bytes once and index a native buffer, or iterate the payload instead of one host read per element",
        );
    }
}
