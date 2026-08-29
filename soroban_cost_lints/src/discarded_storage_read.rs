use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind, Stmt, StmtKind, PatKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    pub DISCARDED_STORAGE_READ,
    Warn,
    "reads from storage whose result is never used"
}

declared_lint_pass!(DiscardedStorageRead => [DISCARDED_STORAGE_READ]);

impl<'tcx> LateLintPass<'tcx> for DiscardedStorageRead {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if is_storage_read(cx, expr) {
            if is_expr_discarded(cx, expr) {
                span_lint_and_help(
                    cx,
                    DISCARDED_STORAGE_READ,
                    expr.span,
                    "storage read result is discarded",
                    None,
                    "delete the storage read or use its returned value",
                );
            }
        }
    }
}

fn is_storage_read(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let ExprKind::MethodCall(path, receiver, _args, _) = expr.kind {
        let name = path.ident.as_str();
        if name == "get" || name == "has" {
            let ty = cx.typeck_results().expr_ty(receiver);
            let ty_str = format!("{:?}", ty);
            if ty_str.contains("storage") || ty_str.contains("Instance") || ty_str.contains("Persistent") || ty_str.contains("Temporary") || is_storage_receiver(cx, receiver) {
                return true;
            }
        }
    }
    false
}

fn is_storage_receiver(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr);
    let s = format!("{:?}", ty);
    s.contains("Storage") || s.contains("Instance") || s.contains("Persistent") || s.contains("Temporary")
}

fn is_expr_discarded(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let hir = cx.tcx.hir();
    let parent_id = hir.parent_id(expr.hir_id);
    let parent_node = hir.find(parent_id);

    match parent_node {
        Some(rustc_hir::Node::Stmt(Stmt { kind: StmtKind::Semi(_), .. })) => true,
        Some(rustc_hir::Node::Local(local)) => {
            if let PatKind::Wild = local.pat.kind {
                true
            } else if let PatKind::Binding(_, _, ident, _) = local.pat.kind {
                // If bound to a variable, check if it's ever used in the same body or function
                let name = ident.as_str();
                if name.starts_with('_') && name != "_" {
                    // Conventionally unused, but let's be precise: if never referenced, treat as discarded
                    !is_local_referenced_in_body(cx, expr, ident.name)
                } else {
                    false
                }
            } else {
                false
            }
        }
        Some(rustc_hir::Node::Expr(parent_expr)) => {
            match parent_expr.kind {
                ExprKind::Block(block, _) => {
                    // If it's the last expression of a block, it might be used by the block's parent, unless the block itself is discarded
                    if block.expr.map_or(false, |e| e.hir_id == expr.hir_id) {
                        is_expr_discarded(cx, parent_expr)
                    } else {
                        true
                    }
                }
                _ => false,
            }
        }
        _ => true,
    }
}

fn is_local_referenced_in_body(_cx: &LateContext<'_>, expr: &Expr<'_>, name: rustc_span::Symbol) -> bool {
    // Simple lexical/hir scope check or conservative fallback
    // For safety, let's walk the enclosing body if possible or assume used if not wild/underscore
    let _ = (expr, name);
    true
}
