use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::def::Res;
use rustc_hir::intravisit::{Visitor, walk_expr};
use rustc_hir::{Expr, ExprKind, HirId, Node, PatKind, QPath, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{DISCARDED_STORAGE_READ, SOROBAN_STORAGE_TYPES, matches_any_path, ty_adt_def};

declare_lint_pass!(DiscardedStorageRead => [DISCARDED_STORAGE_READ]);

impl<'tcx> LateLintPass<'tcx> for DiscardedStorageRead {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if is_storage_read(cx, expr) && is_expr_discarded(cx, expr) {
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

fn is_storage_read(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let ExprKind::MethodCall(path, receiver, _args, _) = expr.kind {
        let name = path.ident.as_str();
        if name == "get" || name == "has" {
            let ty = cx.typeck_results().expr_ty(receiver);
            if ty_adt_def(ty.peel_refs())
                .is_some_and(|adt_def| matches_any_path(cx, adt_def.did(), SOROBAN_STORAGE_TYPES))
            {
                return true;
            }
        }
    }
    false
}

fn is_expr_discarded(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // `TyCtxt::hir()` and `Node::Local` are gone in the pinned nightly:
    // `parent_hir_node` returns the `Node` directly, and a `let` binding is
    // now `Node::LetStmt`.
    let parent_node = cx.tcx.parent_hir_node(expr.hir_id);

    match parent_node {
        rustc_hir::Node::Stmt(Stmt {
            kind: StmtKind::Semi(_),
            ..
        }) => true,
        rustc_hir::Node::LetStmt(local) => {
            if let PatKind::Wild = local.pat.kind {
                true
            } else if let PatKind::Binding(_, _, ident, _) = local.pat.kind {
                // If bound to a variable, check if it's ever used in the same body or function
                let name = ident.as_str();
                if name.starts_with('_') && name != "_" {
                    // Conventionally unused, but let's be precise: if never referenced, treat as discarded
                    !is_binding_referenced(cx, expr, local.pat.hir_id)
                } else {
                    false
                }
            } else {
                false
            }
        }
        rustc_hir::Node::Expr(parent_expr) => {
            match parent_expr.kind {
                ExprKind::Block(block, _) => {
                    // If it's the last expression of a block, it might be used by the block's parent, unless the block itself is discarded
                    if block.expr.is_some_and(|e| e.hir_id == expr.hir_id) {
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

/// Returns `true` if the `_`-prefixed binding created by this statement is
/// referenced anywhere inside its enclosing block.
///
/// Rust scopes a `let` binding to the block that declares it, so walking that
/// block is both complete (no use can live outside it) and conservative: a use
/// in a nested block or closure still counts, because `Res::Local` resolves
/// across both. Any reference at all means the value is live, so the lint
/// stays quiet.
///
/// If the enclosing block cannot be resolved, this returns `true` — a binding
/// whose scope cannot be inspected is assumed to be in use, and the lint
/// rather misses one case than reports a live value as discarded.
fn is_binding_referenced(cx: &LateContext<'_>, expr: &Expr<'_>, binding_id: HirId) -> bool {
    let Some(block) = enclosing_block(cx, expr.hir_id) else {
        return true;
    };

    let mut visitor = BindingUseVisitor {
        target: binding_id,
        found: false,
    };
    visitor.visit_block(block);
    visitor.found
}

/// The closest enclosing `Block` of `id`, if there is one.
fn enclosing_block<'tcx>(
    cx: &LateContext<'tcx>,
    mut id: HirId,
) -> Option<&'tcx rustc_hir::Block<'tcx>> {
    loop {
        match cx.tcx.parent_hir_node(id) {
            Node::Block(block) => return Some(block),
            Node::Stmt(stmt) => id = stmt.hir_id,
            Node::Expr(expr) => id = expr.hir_id,
            Node::LetStmt(local) => id = local.hir_id,
            _ => return None,
        }
    }
}

/// Searches one block for a use of the target binding, stopping at the first
/// one found.
struct BindingUseVisitor {
    target: HirId,
    found: bool,
}

impl<'hir> Visitor<'hir> for BindingUseVisitor {
    fn visit_expr(&mut self, expr: &'hir Expr<'hir>) {
        if self.found {
            return;
        }

        if let ExprKind::Path(QPath::Resolved(None, path)) = expr.kind
            && let Res::Local(id) = path.res
            && id == self.target
        {
            self.found = true;
            return;
        }

        walk_expr(self, expr);
    }
}
