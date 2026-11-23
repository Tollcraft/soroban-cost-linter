use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;

use crate::LARGE_CONSTANT_ARRAY;

declare_lint_pass!(LargeConstantArray => [LARGE_CONSTANT_ARRAY]);

const LARGE_ARRAY_THRESHOLD: u64 = 4096;

impl<'tcx> LateLintPass<'tcx> for LargeConstantArray {
    /// Flags an array literal or repeat expression whose total byte size
    /// exceeds [`LARGE_ARRAY_THRESHOLD`].
    ///
    /// For types with a known layout, the size is computed as element count
    /// multiplied by element size in bytes. For generic elements with no layout,
    /// it falls back to element count and notes this in the diagnostic help.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if !matches!(expr.kind, ExprKind::Array(_) | ExprKind::Repeat(..)) {
            return;
        }
        let ty = cx.typeck_results().expr_ty(expr);
        if let ty::Array(elem_ty, len) = ty.kind()
            && let Some(n) = len.try_to_target_usize(cx.tcx)
        {
            let (size_in_bytes, fallback) = match cx.tcx.layout_of(cx.param_env.and(*elem_ty)) {
                Ok(layout) => (n.saturating_mul(layout.size.bytes()), false),
                Err(_) => (n, true),
            };
            if size_in_bytes > LARGE_ARRAY_THRESHOLD {
                let help = if fallback {
                    "consider using host-managed `Bytes` or persistent storage for large data (layout unknown; size estimated by element count)"
                } else {
                    "consider using host-managed `Bytes` or persistent storage for large data"
                };
                span_lint_and_help(
                    cx,
                    LARGE_CONSTANT_ARRAY,
                    expr.span,
                    "embedding a large array in contract code increases Wasm size",
                    None,
                    help,
                );
            }
        }
    }
}
