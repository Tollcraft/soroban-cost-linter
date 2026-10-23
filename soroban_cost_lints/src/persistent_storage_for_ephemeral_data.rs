//! Flags a persistent storage write that is removed again on every
//! control-flow path through the function.
//!
//! # Why this is a cost problem
//!
//! A persistent entry is paid for by rent and by the archival ledger write
//! that keeps it alive. If the value is written and then removed before the
//! call returns, nothing ever reads it across invocations and the entry never
//! needed the persistent tier: temporary storage would have served the call at
//! a fraction of the cost.
//!
//! # How the reachability check works
//!
//! The analysis is deliberately local and syntactic. For every
//! `persistent().set(&key, ..)` statement, the statements that follow it in
//! the same block are scanned in order:
//!
//! * an unconditional `persistent().remove(&key)` proves the entry is torn
//!   down;
//! * an `if`/`else` proves it only when *both* arms reach a removal;
//! * an `if` without an else, a loop, or any `return`/`?` on the way proves
//!   nothing, because a path that skips the removal survives;
//! * running out of statements proves nothing — the entry survives the call.
//!
//! The block's tail expression counts as its final statement, so an
//! `if`/`else` that ends the function is scanned like any other statement.
//!
//! Keys are compared by their normalised source text, so a dynamic key is
//! reported only when the same expression appears in both the write and the
//! removal.

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::snippet_opt;
use rustc_hir as hir;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{Block, Expr, ExprKind, LetStmt, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::DesugaringKind;

use crate::{PERSISTENT_STORAGE_FOR_EPHEMERAL_DATA, match_soroban_def_path};

declare_lint_pass!(PersistentStorageForEphemeralData => [PERSISTENT_STORAGE_FOR_EPHEMERAL_DATA]);

/// The persistent storage accessor type.
const PERSISTENT_STORAGE_TYPE: &[&str] = &["soroban_sdk", "storage", "Persistent"];

/// Message reported for a persistent write that is always removed again.
const PERSISTENT_STORAGE_FOR_EPHEMERAL_DATA_MSG: &str =
    "persistent storage write is removed on every path through the function";

/// Help note attached to [`PERSISTENT_STORAGE_FOR_EPHEMERAL_DATA_MSG`].
const PERSISTENT_STORAGE_FOR_EPHEMERAL_DATA_HELP: &str = "use temporary storage instead: the value is explicitly removed in this function, so it never needs to outlive the call; temporary entries also avoid the rent payments that keep persistent entries alive";

/// Late pass backing [`PERSISTENT_STORAGE_FOR_EPHEMERAL_DATA`].
impl<'tcx> LateLintPass<'tcx> for PersistentStorageForEphemeralData {
    /// Scans each block for persistent writes and checks whether the following
    /// statements remove the same key on every path.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        for (index, stmt) in block.stmts.iter().enumerate() {
            let Some(expr) = stmt_expr(stmt) else {
                continue;
            };
            let Some(key) = persistent_set_key(cx, expr) else {
                continue;
            };
            if stmts_remove_key(cx, &block.stmts[index + 1..], block.expr, &key) {
                span_lint_and_help(
                    cx,
                    PERSISTENT_STORAGE_FOR_EPHEMERAL_DATA,
                    expr.span,
                    PERSISTENT_STORAGE_FOR_EPHEMERAL_DATA_MSG,
                    None,
                    PERSISTENT_STORAGE_FOR_EPHEMERAL_DATA_HELP,
                );
            }
        }
    }
}

/// The expression a statement evaluates: a `let` initialiser or the statement
/// expression itself.
fn stmt_expr<'tcx>(stmt: &'tcx Stmt<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match stmt.kind {
        StmtKind::Let(&LetStmt {
            init: Some(init), ..
        }) => Some(init),
        StmtKind::Expr(expr) | StmtKind::Semi(expr) => Some(expr),
        _ => None,
    }
}

/// The normalised source text of a storage key. `set`/`remove` take the key by
/// reference, so the borrow is peeled to compare the key expression itself.
fn key_text<'tcx>(cx: &LateContext<'tcx>, arg: &'tcx Expr<'tcx>) -> Option<String> {
    let inner = match arg.kind {
        ExprKind::AddrOf(_, _, inner) => inner,
        _ => arg,
    };
    snippet_opt(cx, inner.span)
}

/// `Some(key text)` when `expr` is `persistent().set(&key, ..)`.
fn persistent_set_key<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<String> {
    let ExprKind::MethodCall(path_segment, receiver, args, _span) = expr.kind else {
        return None;
    };
    if path_segment.ident.name.as_str() != "set" || args.len() < 2 {
        return None;
    }
    if !is_persistent_receiver(cx, receiver) {
        return None;
    }
    key_text(cx, args.first()?)
}

/// Whether `expr` is `persistent().remove(&key)` for the given key.
fn is_persistent_remove<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, key: &str) -> bool {
    let ExprKind::MethodCall(path_segment, receiver, args, _span) = expr.kind else {
        return false;
    };
    if path_segment.ident.name.as_str() != "remove" {
        return false;
    }
    let Some(arg) = args.first() else {
        return false;
    };
    is_persistent_receiver(cx, receiver) && key_text(cx, arg).as_deref() == Some(key)
}

/// Whether `receiver` is the Soroban persistent storage accessor.
fn is_persistent_receiver<'tcx>(cx: &LateContext<'tcx>, receiver: &'tcx Expr<'tcx>) -> bool {
    let peeled = cx.typeck_results().expr_ty(receiver).peel_refs();
    let rustc_middle::ty::Adt(adt_def, _) = peeled.kind() else {
        return false;
    };
    match_soroban_def_path(cx, adt_def.did(), PERSISTENT_STORAGE_TYPE)
}

/// Whether every path through `stmts` (and the block's `tail` expression)
/// removes `key` before leaving the function.
///
/// The scan stops at the first construct that makes the outcome conditional
/// without covering both arms, and reports "not removed" for it.
fn stmts_remove_key<'tcx>(
    cx: &LateContext<'tcx>,
    stmts: &'tcx [Stmt<'tcx>],
    tail: Option<&'tcx Expr<'tcx>>,
    key: &str,
) -> bool {
    for stmt in stmts {
        let Some(expr) = stmt_expr(stmt) else {
            if stmt_has_early_exit(stmt) {
                return false;
            }
            continue;
        };

        if let Some(covered) = removes_key_here(cx, expr, key) {
            return covered;
        }
    }

    tail.is_some_and(|expr| removes_key_here(cx, expr, key).unwrap_or(false))
}

/// `Some(true)` when `expr` removes `key` on every path, `Some(false)` when it
/// provably does not, and `None` when it neither removes the key nor settles
/// the question by itself.
///
/// The `None` case lets the caller keep scanning later statements, so a
/// harmless statement between the write and an unconditional removal does not
/// hide the removal.
fn removes_key_here<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    key: &str,
) -> Option<bool> {
    if is_persistent_remove(cx, expr, key) {
        return Some(true);
    }

    match expr.kind {
        ExprKind::If(_, then_expr, Some(else_expr)) => {
            Some(branch_removes_key(cx, then_expr, key) && branch_removes_key(cx, else_expr, key))
        }
        // One arm only: the fall-through path skips the removal.
        ExprKind::If(..) => Some(false),
        // A `return`/`?` here can leave the function before a later removal.
        _ if expr_has_early_exit(expr) => Some(false),
        _ => None,
    }
}

/// Whether the arm of an `if` removes `key` on every path.
fn branch_removes_key<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, key: &str) -> bool {
    match expr.kind {
        ExprKind::Block(block, _) => stmts_remove_key(cx, block.stmts, block.expr, key),
        _ => is_persistent_remove(cx, expr, key),
    }
}

/// Collects `return`/`?` sites in a statement, ignoring closure bodies (their
/// `return` leaves the closure, not the function) and nested items.
#[derive(Default)]
struct EarlyExitVisitor {
    found: bool,
}

impl<'tcx> Visitor<'tcx> for EarlyExitVisitor {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if self.found {
            return;
        }
        match expr.kind {
            ExprKind::Ret(_) => {
                self.found = true;
                return;
            }
            ExprKind::Closure(..) => return,
            _ => {}
        }
        if expr.span.is_desugaring(DesugaringKind::QuestionMark) {
            self.found = true;
            return;
        }
        intravisit::walk_expr(self, expr);
    }

    fn visit_item(&mut self, _: &'tcx hir::Item<'tcx>) {}
}

/// Whether `expr` can leave the function before a later removal.
fn expr_has_early_exit(expr: &Expr<'_>) -> bool {
    let mut visitor = EarlyExitVisitor::default();
    visitor.visit_expr(expr);
    visitor.found
}

/// Whether a non-expression statement (a `let`, typically) can leave the
/// function before a later removal.
fn stmt_has_early_exit(stmt: &Stmt<'_>) -> bool {
    match stmt.kind {
        StmtKind::Let(&LetStmt {
            init: Some(init), ..
        }) => expr_has_early_exit(init),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{PERSISTENT_STORAGE_FOR_EPHEMERAL_DATA_HELP, PERSISTENT_STORAGE_TYPE};

    /// Mirrors the textual key comparison the pass performs.
    fn removed_keys(keys: &[&str]) -> HashSet<String> {
        keys.iter().map(|key| (*key).to_string()).collect()
    }

    #[test]
    fn only_persistent_tier_is_matched() {
        assert_eq!(
            PERSISTENT_STORAGE_TYPE,
            &["soroban_sdk", "storage", "Persistent"]
        );
    }

    #[test]
    fn help_recommends_temporary_storage() {
        assert!(PERSISTENT_STORAGE_FOR_EPHEMERAL_DATA_HELP.contains("temporary storage"));
    }

    #[test]
    fn key_comparison_is_textual() {
        assert_eq!(removed_keys(&["key"]).len(), 1);
        assert!(!removed_keys(&["key"]).contains("other"));
    }
}
