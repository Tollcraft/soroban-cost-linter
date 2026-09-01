use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::higher;
use rustc_hir::intravisit::{Visitor, walk_expr};
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::UNBOUNDED_INPUT_LOOP;

declare_lint_pass!(UnboundedInputLoop => [UNBOUNDED_INPUT_LOOP]);

const WRITE_METHODS: &[&str] = &["set", "put", "make_persistent"];
const ITERATING_METHODS: &[&str] = &["iter", "iter_combined"];
const COLLECTION_SUFFIXES: &[&str] = &["Vec", "Map", "Array", "Bytes", "Set"];

impl<'tcx> LateLintPass<'tcx> for UnboundedInputLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Only `for` loops over a caller-supplied collection are recognized
        // as input-driven iteration. `while`/`loop` forms are conservatively
        // skipped to avoid flagging intentionally bounded loops.
        let Some(higher::ForLoop { arg: iter, .. }) = higher::ForLoop::hir(expr) else {
            return;
        };

        if !iterates_over_input(cx, iter) {
            return;
        }

        if !contains_storage_write(cx, expr) {
            return;
        }

        span_lint_and_help(
            cx,
            UNBOUNDED_INPUT_LOOP,
            expr.span,
            "loop is driven by caller-supplied input and performs storage writes \
             inside the loop body",
            None,
            "the number of iterations is controlled by the caller and each iteration \
             performs a storage write; bound the loop (e.g. cap iterations or batch the \
             writes) so a single invocation cannot exhaust the budget",
        );
    }
}

/// Returns `true` when `iter` yields values whose count depends on a caller
/// supplied collection. A `for` loop over a `Vec`/`Map`/etc. iterator counts as
/// input-driven because the collection size is set by the caller.
fn iterates_over_input(cx: &LateContext<'_>, iter: &Expr<'_>) -> bool {
    match iter.kind {
        ExprKind::MethodCall(path, receiver, _args, _) => {
            // e.g. `for x in input.iter()` / `input.iter_combined(...)`
            ITERATING_METHODS.contains(&path.ident.as_str()) && is_collection(cx, receiver)
        }
        ExprKind::Path(QPath::Resolved(_, _)) | ExprKind::AddrOf(..) => {
            // e.g. `for x in &input` / `for x in input`
            is_collection(cx, iter)
        }
        ExprKind::Call(_, args) => {
            // e.g. `for x in 0..input.len()`
            args.iter().any(|arg| is_input_driven(cx, arg))
        }
        _ => false,
    }
}

/// Does this expression reference `.len()` of a caller-supplied collection, or
/// a caller-supplied collection directly?
fn is_input_driven(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    match expr.kind {
        ExprKind::MethodCall(path, receiver, _args, _) => {
            path.ident.as_str() == "len" && is_collection(cx, receiver)
        }
        _ => is_collection(cx, expr),
    }
}

/// Conservatively treat any value whose type name matches a known Soroban
/// collection name as being supplied by the caller. This keeps false negatives
/// low at the cost of some over-reporting on non-parameter collections.
fn is_collection(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr).peel_refs();
    let ty_str = format!("{ty:?}");
    COLLECTION_SUFFIXES
        .iter()
        .any(|suffix| ty_str.contains(suffix) || ty_str.ends_with(suffix))
}

/// Walks the loop body looking for a storage write method (`set`, `put`,
/// `make_persistent`) called on a storage-typed receiver.
fn contains_storage_write<'tcx>(cx: &LateContext<'tcx>, loop_expr: &'tcx Expr<'tcx>) -> bool {
    if let Some(higher::ForLoop { body, .. }) = higher::ForLoop::hir(loop_expr) {
        let mut v = WriteVisitor { cx, found: false };
        v.visit_expr(body);
        v.found
    } else {
        false
    }
}

struct WriteVisitor<'v, 'tcx> {
    cx: &'v LateContext<'tcx>,
    found: bool,
}

impl<'v, 'tcx> Visitor<'tcx> for WriteVisitor<'v, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if self.found {
            return;
        }
        if let ExprKind::MethodCall(path, receiver, _args, _) = expr.kind
            && WRITE_METHODS.contains(&path.ident.as_str())
            && is_storage_receiver(self.cx, receiver)
        {
            self.found = true;
            return;
        }
        walk_expr(self, expr);
    }
}

fn is_storage_receiver(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr);
    let s = format!("{ty:?}");
    s.contains("Storage")
        || s.contains("Instance")
        || s.contains("Persistent")
        || s.contains("Temporary")
}
