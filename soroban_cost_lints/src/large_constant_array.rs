use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;

use crate::LARGE_CONSTANT_ARRAY;

declare_lint_pass!(LargeConstantArray => [LARGE_CONSTANT_ARRAY]);

const LARGE_ARRAY_THRESHOLD: u64 = 4096;

impl<'tcx> LateLintPass<'tcx> for LargeConstantArray {
    /// Flags an array literal whose element count exceeds
    /// [`LARGE_ARRAY_THRESHOLD`].
    ///
    /// Only a *literal* is reported. An array literal lists every element, so
    /// all of them end up in the contract's data section and the deployed
    /// Wasm binary grows with the array. A repeated fill such as
    /// `[0u8; 8192]` is not reported: it lowers to a zeroed region and adds
    /// nothing to the binary.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if !matches!(expr.kind, ExprKind::Array(_)) {
            return;
        }
        let ty = cx.typeck_results().expr_ty(expr);
        if let ty::Array(_elem_ty, len) = ty.kind()
            && let Some(n) = len.try_to_target_usize(cx.tcx)
            && n > LARGE_ARRAY_THRESHOLD
        {
            span_lint_and_help(
                cx,
                LARGE_CONSTANT_ARRAY,
                expr.span,
                "embedding a large array in contract code increases Wasm size",
                None,
                "consider using host-managed `Bytes` or persistent storage for large data",
            );
        }
    }
}
