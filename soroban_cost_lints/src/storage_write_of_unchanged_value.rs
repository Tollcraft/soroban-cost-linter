//! `storage_write_of_unchanged_value` — flags a storage write whose written
//! value is identical to the value previously read from the same storage key.
//!
//! Reading a storage key, leaving the value unchanged, and writing it back
//! re-encodes and rewrites the ledger entry, paying a metered write and rent
//! for an identical value; a comparison guard avoids this cost.

use std::collections::HashMap;

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::snippet_opt;
use rustc_hir as hir;
use rustc_hir::def::Res;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{BinOpKind, Body, Expr, ExprKind, HirId, Node, PatKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;
use rustc_span::Span;

use crate::{STORAGE_WRITE_OF_UNCHANGED_VALUE, match_soroban_def_path};

declare_lint_pass!(StorageWriteOfUnchangedValue => [STORAGE_WRITE_OF_UNCHANGED_VALUE]);

const STORAGE_ACCESSOR_TIERS: [(&[&str], &str); 3] = [
    (&["soroban_sdk", "storage", "Instance"], "instance"),
    (&["soroban_sdk", "storage", "Persistent"], "persistent"),
    (&["soroban_sdk", "storage", "Temporary"], "temporary"),
];

const LINT_MSG: &str = "writing an unchanged value back to storage";
const LINT_HELP: &str = "guard the write with an inequality check (`if old_val != new_val`) or avoid rewriting identical data";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct StorageKey {
    tier: &'static str,
    key_text: String,
}

#[derive(Debug, Clone)]
struct TrackedRead {
    key: StorageKey,
    _span: Span,
}

impl<'tcx> LateLintPass<'tcx> for StorageWriteOfUnchangedValue {
    fn check_body(&mut self, cx: &LateContext<'tcx>, body: &Body<'tcx>) {
        let mut visitor = StorageWriteVisitor {
            cx,
            tracked_reads: HashMap::new(),
            body,
        };
        visitor.visit_body(body);
    }
}

struct StorageWriteVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    tracked_reads: HashMap<HirId, TrackedRead>,
    body: &'a Body<'tcx>,
}

impl<'tcx> Visitor<'tcx> for StorageWriteVisitor<'_, 'tcx> {
    fn visit_stmt(&mut self, stmt: &'tcx hir::Stmt<'tcx>) {
        if let hir::StmtKind::Let(local) = stmt.kind
            && let Some(init) = local.init
            && let Some((key, span)) = extract_storage_read(self.cx, init)
        {
            // Track the bound local variable(s)
            record_pat_bindings(local.pat, &mut self.tracked_reads, key, span);
        }
        intravisit::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let Some((tier, key_text, val_expr, span)) = extract_storage_write(self.cx, expr) {
            let write_key = StorageKey { tier, key_text };
            let peeled_val = peel_addr_and_deref(val_expr);
            if let Some(local_id) = res_local_hir_id(peeled_val)
                && let Some(read) = self.tracked_reads.get(&local_id)
                && read.key == write_key
            {
                // Verify the local variable was not mutated after read
                if !is_mutated(self.body, local_id) && !is_guarded_by_ne(self.cx, expr, local_id) {
                    span_lint_and_help(
                        self.cx,
                        STORAGE_WRITE_OF_UNCHANGED_VALUE,
                        span,
                        LINT_MSG,
                        None,
                        LINT_HELP,
                    );
                }
            }
        }
        intravisit::walk_expr(self, expr);
    }
}

fn record_pat_bindings(
    pat: &hir::Pat<'_>,
    map: &mut HashMap<HirId, TrackedRead>,
    key: StorageKey,
    span: Span,
) {
    match pat.kind {
        PatKind::Binding(_, hir_id, _, _) => {
            map.insert(hir_id, TrackedRead { key, _span: span });
        }
        PatKind::Tuple(pats, _) | PatKind::Slice(pats, _, _) => {
            for p in pats {
                record_pat_bindings(p, map, key.clone(), span);
            }
        }
        PatKind::TupleStruct(_, pats, _) => {
            for p in pats {
                record_pat_bindings(p, map, key.clone(), span);
            }
        }
        _ => {}
    }
}

fn extract_storage_read<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(StorageKey, Span)> {
    let mut cur = expr;
    // Unwrap chained calls like `.unwrap()`, `.expect(...)`, `.ok()`, `.unwrap_or(...)`
    while let ExprKind::MethodCall(seg, receiver, _, _) = cur.kind {
        let name = seg.ident.as_str();
        if name == "unwrap" || name == "expect" || name == "ok" || name == "unwrap_or_default" {
            cur = receiver;
        } else {
            break;
        }
    }

    if let ExprKind::MethodCall(seg, receiver, args, span) = cur.kind {
        let name = seg.ident.as_str();
        if (name == "get" || name == "try_get") && !args.is_empty() {
            let tier = storage_tier_of(cx, receiver)?;
            let key_text = extract_key_snippet(cx, &args[0])?;
            return Some((StorageKey { tier, key_text }, span));
        }
    }
    None
}

fn extract_storage_write<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'static str, String, &'tcx Expr<'tcx>, Span)> {
    if let ExprKind::MethodCall(seg, receiver, args, _span) = expr.kind
        && seg.ident.as_str() == "set"
        && args.len() >= 2
    {
        let tier = storage_tier_of(cx, receiver)?;
        let key_text = extract_key_snippet(cx, &args[0])?;
        let val_expr = &args[1];
        return Some((tier, key_text, val_expr, expr.span));
    }
    None
}

fn extract_key_snippet(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<String> {
    let inner = if let ExprKind::AddrOf(_, _, e) = expr.kind {
        e
    } else {
        expr
    };
    snippet_opt(cx, inner.span)
}

fn storage_tier_of<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> Option<&'static str> {
    let peeled = cx.typeck_results().expr_ty(expr).peel_refs();
    let ty::Adt(adt_def, _) = peeled.kind() else {
        return None;
    };
    let def_id = adt_def.did();
    STORAGE_ACCESSOR_TIERS
        .iter()
        .find(|(segments, _)| match_soroban_def_path(cx, def_id, segments))
        .map(|(_, tier)| *tier)
}

fn peel_addr_and_deref<'tcx>(mut expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    loop {
        match expr.kind {
            ExprKind::AddrOf(_, _, inner) | ExprKind::Unary(hir::UnOp::Deref, inner) => {
                expr = inner;
            }
            _ => break expr,
        }
    }
}

fn res_local_hir_id(expr: &Expr<'_>) -> Option<HirId> {
    if let ExprKind::Path(QPath::Resolved(_, path)) = expr.kind
        && let Res::Local(id) = path.res
    {
        Some(id)
    } else {
        None
    }
}

fn is_mutated(body: &Body<'_>, target: HirId) -> bool {
    struct MutationChecker {
        target: HirId,
        mutated: bool,
    }
    impl<'tcx> Visitor<'tcx> for MutationChecker {
        fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
            if let ExprKind::Assign(lhs, _, _) | ExprKind::AssignOp(_, lhs, _) = expr.kind
                && res_local_hir_id(peel_addr_and_deref(lhs)) == Some(self.target)
            {
                self.mutated = true;
            }
            intravisit::walk_expr(self, expr);
        }
    }
    let mut checker = MutationChecker {
        target,
        mutated: false,
    };
    checker.visit_body(body);
    checker.mutated
}

fn is_guarded_by_ne<'tcx>(
    cx: &LateContext<'tcx>,
    write_expr: &'tcx Expr<'tcx>,
    target_id: HirId,
) -> bool {
    let mut current_id = write_expr.hir_id;
    for (parent_id, node) in cx.tcx.hir_parent_iter(current_id) {
        if let Node::Expr(parent_expr) = node
            && let ExprKind::If(cond, then_expr, _) = parent_expr.kind
            && expr_contains_hir_id(then_expr, current_id)
            && cond_mentions_ne(cond, target_id)
        {
            return true;
        }
        current_id = parent_id;
    }
    false
}

fn expr_contains_hir_id(root: &Expr<'_>, target: HirId) -> bool {
    struct SearchVisitor {
        target: HirId,
        found: bool,
    }
    impl<'tcx> Visitor<'tcx> for SearchVisitor {
        fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
            if expr.hir_id == self.target {
                self.found = true;
            }
            intravisit::walk_expr(self, expr);
        }
    }
    let mut visitor = SearchVisitor {
        target,
        found: false,
    };
    visitor.visit_expr(root);
    visitor.found
}

fn cond_mentions_ne(cond: &Expr<'_>, target_id: HirId) -> bool {
    struct NeVisitor {
        target: HirId,
        found_ne: bool,
    }
    impl<'tcx> Visitor<'tcx> for NeVisitor {
        fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
            if let ExprKind::Binary(op, left, right) = expr.kind
                && op.node == BinOpKind::Ne
            {
                let left_id = res_local_hir_id(peel_addr_and_deref(left));
                let right_id = res_local_hir_id(peel_addr_and_deref(right));
                if left_id == Some(self.target) || right_id == Some(self.target) {
                    self.found_ne = true;
                }
            }
            intravisit::walk_expr(self, expr);
        }
    }
    let mut v = NeVisitor {
        target: target_id,
        found_ne: false,
    };
    v.visit_expr(cond);
    v.found_ne
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_are_pinned() {
        assert_eq!(LINT_MSG, "writing an unchanged value back to storage");
        assert_eq!(
            LINT_HELP,
            "guard the write with an inequality check (`if old_val != new_val`) or avoid rewriting identical data"
        );
    }

    #[test]
    fn tier_table_names_the_three_storage_accessors() {
        let tiers: Vec<&str> = STORAGE_ACCESSOR_TIERS
            .iter()
            .map(|(_, tier)| *tier)
            .collect();
        assert_eq!(tiers, ["instance", "persistent", "temporary"]);
        for (segments, _) in STORAGE_ACCESSOR_TIERS {
            assert_eq!(segments.first(), Some(&"soroban_sdk"));
        }
    }
}
