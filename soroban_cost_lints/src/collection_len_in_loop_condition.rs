//! `collection_len_in_loop_condition` — flags `len()` on a Soroban collection
//! when it is re-evaluated by a `while` loop condition.
//!
//! `.len()` on a Soroban `Vec` or `Map` is a host call. In
//! `while i < vec.len() { .. }` the condition is evaluated on every iteration,
//! so the same length is fetched — and paid for — once per iteration even
//! though it cannot change when the loop never mutates the collection.
//! Hoisting it into a local before the loop makes the host call happen once.
//!
//! Detection:
//!
//! 1. the expression is a `while` loop, with the condition as the block's tail
//!    expression,
//! 2. the condition contains a `len` method call whose receiver resolves to a
//!    type in [`SOROBAN_COLLECTION_TYPES`] (native `std` collections are out of
//!    scope), and
//! 3. the receiver is not mutated inside the loop body or the condition.
//!
//! `for x in 0..vec.len()` is deliberately not reported: a `for` range is
//! evaluated once, so there is nothing to hoist.

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::res::MaybeResPath;
use rustc_hir as hir;
use rustc_hir::intravisit::{Visitor, walk_expr};
use rustc_hir::{Expr, ExprKind, HirId, LoopSource};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{COLLECTION_LEN_IN_LOOP_CONDITION, SOROBAN_COLLECTION_TYPES, is_type_match};

declare_lint_pass!(CollectionLenInLoopCondition => [COLLECTION_LEN_IN_LOOP_CONDITION]);

/// Methods that mutate a Soroban collection. A condition that is re-evaluated
/// after one of these runs is genuinely looking at a changing length, so the
/// lint stays quiet for that receiver.
const MUTATION_METHODS: &[&str] = &[
    "append",
    "clear",
    "extend_from_array",
    "insert",
    "pop_back",
    "pop_front",
    "push_back",
    "push_front",
    "remove",
    "resize",
    "set",
    "swap_remove",
    "truncate",
    "update",
];

/// Message reported for a `len()` call in a `while` condition.
const COLLECTION_LEN_IN_LOOP_CONDITION_MSG: &str =
    "collection len() called in a loop condition without mutation";

/// Help note attached to [`COLLECTION_LEN_IN_LOOP_CONDITION_MSG`].
const COLLECTION_LEN_IN_LOOP_CONDITION_HELP: &str = "hoist/bind the collection's length into a \
     local variable before the loop starts, and compare against that local in the while \
     condition instead of calling .len() each iteration.";

/// Late pass backing [`COLLECTION_LEN_IN_LOOP_CONDITION`].
impl<'tcx> LateLintPass<'tcx> for CollectionLenInLoopCondition {
    /// Flags a `len()` call on a Soroban collection that sits in the condition
    /// of a `while` loop whose iterations do not mutate the collection.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::Loop(block, _label, LoopSource::While, _span) = expr.kind else {
            return;
        };
        // HIR keeps a `while` condition as the block's tail expression.
        let Some(cond) = block.expr else {
            return;
        };

        let mut finder = LenCallFinder {
            cx,
            calls: Vec::new(),
        };
        finder.visit_expr(cond);
        for call in finder.calls {
            if receiver_is_mutated_in(call, block) {
                continue;
            }
            span_lint_and_help(
                cx,
                COLLECTION_LEN_IN_LOOP_CONDITION,
                call.span,
                COLLECTION_LEN_IN_LOOP_CONDITION_MSG,
                None,
                COLLECTION_LEN_IN_LOOP_CONDITION_HELP,
            );
        }
    }
}

/// Collects `len()` calls on Soroban collections out of one expression tree.
struct LenCallFinder<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    calls: Vec<&'tcx Expr<'tcx>>,
}

impl<'tcx> Visitor<'tcx> for LenCallFinder<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::MethodCall(path_segment, receiver, _args, _) = expr.kind
            && path_segment.ident.name.as_str() == "len"
            && is_type_match(
                self.cx,
                self.cx.typeck_results().expr_ty(receiver),
                SOROBAN_COLLECTION_TYPES,
            )
        {
            self.calls.push(expr);
        }
        walk_expr(self, expr);
    }
}

/// Whether the receiver of `len_call` is mutated anywhere in `block`.
///
/// `block` is the whole `while` body plus its condition (the tail
/// expression), so a mutation in either place suppresses the lint. A receiver
/// that is not a plain local binding cannot be tracked, and is treated as
/// unmutated — the documented limitation of this structural check.
fn receiver_is_mutated_in(len_call: &Expr<'_>, block: &hir::Block<'_>) -> bool {
    let ExprKind::MethodCall(_, receiver, _, _) = len_call.kind else {
        return false;
    };
    let Some(target) = receiver.res_local_id() else {
        return false;
    };

    let mut visitor = MutationVisitor {
        target,
        mutated: false,
    };
    visitor.visit_block(block);
    visitor.mutated
}

/// Searches a `while` block for a mutation of one local binding.
struct MutationVisitor {
    target: HirId,
    mutated: bool,
}

impl<'hir> Visitor<'hir> for MutationVisitor {
    fn visit_expr(&mut self, expr: &'hir Expr<'hir>) {
        if self.mutated {
            return;
        }

        match expr.kind {
            // `vec.push_back(..)`, `map.set(..)`, ... take `&mut self`.
            ExprKind::MethodCall(path_segment, receiver, _, _)
                if MUTATION_METHODS.contains(&path_segment.ident.name.as_str())
                    && receiver.res_local_id() == Some(self.target) =>
            {
                self.mutated = true;
                return;
            }
            // `vec = other;` replaces the value the condition reads.
            ExprKind::Assign(lhs, _, _) if lhs.res_local_id() == Some(self.target) => {
                self.mutated = true;
                return;
            }
            _ => {}
        }

        walk_expr(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use super::MUTATION_METHODS;

    /// The suppression depends on this list covering the collection-mutating
    /// methods the lint documents; a shrinking list would reintroduce false
    /// positives for mutated collections.
    #[test]
    fn mutation_methods_cover_the_documented_growth_and_shrink_calls() {
        for method in [
            "append",
            "clear",
            "insert",
            "pop_back",
            "push_back",
            "remove",
            "set",
            "truncate",
        ] {
            assert!(MUTATION_METHODS.contains(&method), "missing {method}");
        }
        assert!(!MUTATION_METHODS.contains(&"len"));
        assert!(!MUTATION_METHODS.contains(&"get"));
    }
}
