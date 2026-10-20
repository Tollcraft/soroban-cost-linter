//! Flags a storage key that is read, modified and written back more than once
//! in a single function body.
//!
//! # Why this is a cost problem
//!
//! One read-modify-write cycle is one metered storage read plus one metered
//! storage write. Doing that cycle a second time on the same key in the same
//! body means the accumulator was re-read from storage instead of reused:
//! between the two reads the value never left the host, so the second read and
//! the second write buy nothing. On a balance update touched by three cycles
//! that is six storage operations where two would do.
//!
//! The sibling lints do not catch this shape: `soroban_redundant_storage_read`
//! reports two reads with no intervening write, and `storage_write_without_read`
//! reports a write whose key is never read at all — a body that alternates
//! read/write/read/write is clean for both while still paying double.
//!
//! # Detection
//!
//! `check_body` walks the top-level statements of one function body in source
//! order and tracks a small state machine per `(tier, key)` pair:
//!
//! 1. A `get` on a recognised storage tier object — `Instance`, `Persistent`
//!    or `Temporary` — opens a pending read for the key.
//! 2. A `set` on the same tier closes it: one cycle completes. Every
//!    completion from the second one onward is reported, so a three-cycle body
//!    warns on its second and third writes.
//! 3. Any other call that could touch storage — an unrecognised method, a
//!    plain function call, a helper invocation — conservatively resets every
//!    key, because the call may legitimately have modified any of them. This
//!    is what keeps a body with an intervening helper call quiet.
//!
//! Keys are identified by the tier plus the source text of the key expression
//! (its leading `&` stripped), so two `&1` literals on the same tier are one
//! key, the same literal on different tiers are two keys, and different
//! literals never merge. Only the top level of a body is tracked: nested
//! closures and cross-function flows are out of scope, which keeps the pass a
//! single linear scan with no dataflow.
//!
//! `has` deliberately does not open a cycle: it produces no value to modify,
//! so it can never be the reading half of a read-modify-write — and as any
//! other storage-capable call it conservatively breaks one.

use std::collections::HashMap;

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::snippet_opt;
use rustc_hir::{Block, Body, Expr, ExprKind, LetStmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;
use rustc_span::Span;

use crate::{STORAGE_READ_MODIFY_WRITE, match_soroban_def_path};

declare_lint_pass!(StorageReadModifyWrite => [STORAGE_READ_MODIFY_WRITE]);

/// The storage reads that open a cycle. Only `get` qualifies: it hands the
/// caller the value to modify. `has` yields no value, so it cannot be the
/// reading half of a read-modify-write — and as any other storage-capable
/// call it conservatively breaks one.
const READ_METHODS: &[&str] = &["get"];

/// The storage write that closes a cycle.
const WRITE_METHOD: &str = "set";

/// The three storage accessor types as `(definition path, tier name)` pairs.
const STORAGE_ACCESSOR_TIERS: [(&[&str], &str); 3] = [
    (&["soroban_sdk", "storage", "Instance"], "instance"),
    (&["soroban_sdk", "storage", "Persistent"], "persistent"),
    (&["soroban_sdk", "storage", "Temporary"], "temporary"),
];

/// The warning's message; the wording is pinned by the ui golden.
const STORAGE_READ_MODIFY_WRITE_MSG: &str = "storage read-modify-write cycle: this key was already read, modified, and written once; repeating the cycle wastes storage operations";

/// The help's text; pinned by the ui golden alongside the message.
const STORAGE_READ_MODIFY_WRITE_HELP: &str =
    "reuse the value from the first cycle instead of re-reading and re-writing the same key";

/// A storage key: the tier it lives on plus the source text of the key
/// expression. Two keys are the same when both halves match.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct StorageKey {
    tier: &'static str,
    text: String,
}

/// The cycle state of one key within one function body.
#[derive(Debug, Clone, Default)]
struct KeyState {
    /// Completed read-modify-write cycles seen so far.
    cycles: usize,
    /// A `get` happened and no matching `set` has closed it yet.
    pending_read: bool,
}

/// A storage operation this pass recognises.
enum StorageOp {
    /// A `get` on `key`, opening a cycle.
    Read(StorageKey),
    /// A `set` of `key`, whose call expression spans `span`.
    Write { span: Span, key: StorageKey },
}

impl<'tcx> LateLintPass<'tcx> for StorageReadModifyWrite {
    /// Tracks read-modify-write cycles over the top-level statements of one
    /// function body and reports every `set` from the second cycle onward.
    fn check_body(&mut self, cx: &LateContext<'tcx>, body: &Body<'tcx>) {
        if let ExprKind::Block(block, _) = body.value.kind {
            check_block(cx, block);
        }
    }
}

/// Walks the top-level statements of a body in source order, maintaining the
/// per-key cycle state machine.
fn check_block<'tcx>(cx: &LateContext<'tcx>, block: &Block<'tcx>) {
    let mut key_states: HashMap<StorageKey, KeyState> = HashMap::new();

    let exprs = block
        .stmts
        .iter()
        .filter_map(|stmt| match stmt.kind {
            StmtKind::Let(&LetStmt {
                init: Some(init), ..
            }) => Some(init),
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => Some(expr),
            _ => None,
        })
        .chain(block.expr);

    for expr in exprs {
        process_expr(cx, expr, &mut key_states);
    }
}

/// Applies one top-level expression to the cycle state: a recognised storage
/// read or write advances its key's state machine, and any other expression
/// that could touch storage resets every key.
fn process_expr<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    key_states: &mut HashMap<StorageKey, KeyState>,
) {
    match extract_storage_op(cx, expr) {
        Some(StorageOp::Read(key)) => {
            key_states.entry(key).or_default().pending_read = true;
        }
        Some(StorageOp::Write { span, key }) => {
            let state = key_states.entry(key).or_default();
            if state.pending_read {
                state.cycles += 1;
                state.pending_read = false;
                if state.cycles >= 2 {
                    span_lint_and_help(
                        cx,
                        STORAGE_READ_MODIFY_WRITE,
                        span,
                        STORAGE_READ_MODIFY_WRITE_MSG,
                        None,
                        STORAGE_READ_MODIFY_WRITE_HELP,
                    );
                }
            }
        }
        // Not a direct storage op — this could be a call that touches
        // storage. Conservative: reset all tracked keys.
        None if is_potential_storage_call(cx, expr) => {
            for state in key_states.values_mut() {
                *state = KeyState::default();
            }
        }
        None => {}
    }
}

/// Recognises a storage read or write at the top level of a body, extracting
/// the key (and the write's span) together with the tier it lives on.
fn extract_storage_op<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> Option<StorageOp> {
    let ExprKind::MethodCall(segment, receiver, args, _) = expr.kind else {
        return None;
    };

    let method = segment.ident.as_str();
    if !READ_METHODS.contains(&method) && method != WRITE_METHOD {
        return None;
    }

    let tier = storage_tier_of(cx, receiver)?;
    let key_expr = args.first()?;
    // `get`/`set` take the key by reference; strip the `&` so the text of the
    // key itself is compared rather than the borrow expression.
    let key_inner = if let ExprKind::AddrOf(_, _, inner) = key_expr.kind {
        inner
    } else {
        key_expr
    };
    let key = StorageKey {
        tier,
        text: snippet_opt(cx, key_inner.span)?,
    };

    if method == WRITE_METHOD {
        Some(StorageOp::Write {
            span: expr.span,
            key,
        })
    } else {
        Some(StorageOp::Read(key))
    }
}

/// The tier a receiver belongs to — `Some("instance" | "persistent" |
/// "temporary")` only when its type is one of the three storage accessor
/// types, after peeling references.
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

/// Conservatively determines whether an expression might call code that
/// touches storage, and therefore breaks every tracked cycle.
fn is_potential_storage_call<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> bool {
    match expr.kind {
        ExprKind::Call(_, _) => true,
        ExprKind::MethodCall(_, receiver, _, _) => !is_primitive_receiver(receiver),
        ExprKind::If(_, then, else_expr) => {
            is_potential_storage_call(cx, then)
                || else_expr.is_some_and(|e| is_potential_storage_call(cx, e))
        }
        ExprKind::Block(block, _) => block_has_non_storage_call(cx, block),
        _ => false,
    }
}

/// Whether any top-level statement of a block holds a call that could touch
/// storage.
fn block_has_non_storage_call<'tcx>(cx: &LateContext<'tcx>, block: &Block<'tcx>) -> bool {
    for stmt in block.stmts {
        let expr = match stmt.kind {
            StmtKind::Let(&LetStmt {
                init: Some(init), ..
            }) => init,
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => expr,
            _ => continue,
        };
        if is_potential_storage_call(cx, expr) {
            return true;
        }
    }
    block.expr.is_some_and(|e| is_potential_storage_call(cx, e))
}

/// Whether the receiver of a method call is a literal — a call on a literal
/// (`1.to_string()`) cannot be storage access.
fn is_primitive_receiver(expr: &Expr<'_>) -> bool {
    matches!(expr.kind, ExprKind::Lit(_))
}

#[cfg(test)]
mod tests {
    use super::{
        READ_METHODS, STORAGE_ACCESSOR_TIERS, STORAGE_READ_MODIFY_WRITE_HELP,
        STORAGE_READ_MODIFY_WRITE_MSG, WRITE_METHOD,
    };

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

    /// The ui golden records these strings verbatim; changing either is a
    /// golden re-record, not a refactor.
    #[test]
    fn message_and_help_are_pinned_to_the_ui_golden() {
        assert_eq!(
            STORAGE_READ_MODIFY_WRITE_MSG,
            "storage read-modify-write cycle: this key was already read, modified, and written once; repeating the cycle wastes storage operations"
        );
        assert_eq!(
            STORAGE_READ_MODIFY_WRITE_HELP,
            "reuse the value from the first cycle instead of re-reading and re-writing the same key"
        );
    }

    /// Only `get` opens a cycle — `has` yields no value to modify — and only
    /// `set` closes one. A typo here silently disables the lint.
    #[test]
    fn method_tables_are_the_two_cycle_halves() {
        assert_eq!(READ_METHODS, ["get"]);
        assert_eq!(WRITE_METHOD, "set");
    }
}
