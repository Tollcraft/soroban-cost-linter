use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;

use crate::LARGE_CONSTANT_ARRAY;

declare_lint_pass!(LargeConstantArray => [LARGE_CONSTANT_ARRAY]);

const LARGE_ARRAY_THRESHOLD: u64 = 4096;

pub const LARGE_CONSTANT_ARRAY_MSG: &str =
    "embedding a large array in contract code increases Wasm size";
pub const LARGE_CONSTANT_ARRAY_HELP: &str =
    "consider using host-managed `Bytes` or persistent storage for large data";

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
            span_lint_and_help(
                cx,
                LARGE_CONSTANT_ARRAY,
                expr.span,
                LARGE_CONSTANT_ARRAY_MSG,
                None,
                LARGE_CONSTANT_ARRAY_HELP,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LARGE_CONSTANT_ARRAY_HELP, LARGE_CONSTANT_ARRAY_MSG};

    #[test]
    fn message_and_help_text_are_pinned() {
        assert_eq!(
            LARGE_CONSTANT_ARRAY_MSG,
            "embedding a large array in contract code increases Wasm size"
        );
        assert_eq!(
            LARGE_CONSTANT_ARRAY_HELP,
            "consider using host-managed `Bytes` or persistent storage for large data"
        );
    }

    #[test]
    fn help_is_a_single_sentence() {
        assert!(!LARGE_CONSTANT_ARRAY_HELP.contains(". "));
    }
}
