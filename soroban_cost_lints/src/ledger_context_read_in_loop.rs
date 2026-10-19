use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::get_enclosing_loop_or_multi_call_closure;
use rustc_hir::def::Res;
use rustc_hir::{Expr, ExprKind, HirId, PatKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{LEDGER_CONTEXT_READ_IN_LOOP, matches_any_path, ty_adt_def};

declare_lint_pass!(LedgerContextReadInLoop => [LEDGER_CONTEXT_READ_IN_LOOP]);

const LEDGER_READ_METHODS: &[&str] = &[
    "sequence",
    "timestamp",
    "network_id",
    "protocol_version",
    // soroban-sdk 26.1.1 ledger.rs:73 and 28.0.0 ledger.rs:73: a fifth
    // invariant accessor on the same host object (#871).
    "max_live_until_ledger",
];

/// `soroban_sdk::Env` — the def-path form the Env check now requires, so a
/// local type whose printed name merely contains `Env` no longer satisfies it
/// (#795).
const SOROBAN_ENV_TYPES: &[&[&str]] = &[&["soroban_sdk", "Env"]];

/// How many binding hops a receiver may be resolved through: `let l =
/// env.ledger(); ... l.timestamp()` is one hop. Bounded so a pathological
/// chain cannot walk the whole body.
const MAX_BINDINGS_HOPS: usize = 3;

impl<'tcx> LateLintPass<'tcx> for LedgerContextReadInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Look for method calls: <ledger>.<accessor>()
        if let rustc_hir::ExprKind::MethodCall(path, receiver, _args, _) = expr.kind {
            let method_name = path.ident.as_str();
            // Check if this is a ledger context accessor
            if !LEDGER_READ_METHODS.contains(&method_name) {
                return;
            }
            // The receiver must resolve to a ledger() call on Env — inline or
            // through a simple `let` binding (#795).
            if !is_ledger_receiver(cx, receiver) {
                return;
            }
            // Check if this is inside a loop
            if get_enclosing_loop_or_multi_call_closure(cx, expr).is_some() {
                let help = format!(
                    "ledger context values ({method_name}) are invariant during a single \
                     invocation; hoist this read outside the loop to avoid repeated host calls"
                );
                span_lint_and_help(
                    cx,
                    LEDGER_CONTEXT_READ_IN_LOOP,
                    expr.span,
                    format!(
                        "reading ledger context `{method_name}` inside a loop — the value \
                         cannot change during this invocation"
                    ),
                    None,
                    help,
                );
            }
        }
    }
}

/// Returns `true` when `expr` will evaluate to the `Ledger` context value:
/// either the literal `env.ledger()` call or a local binding whose initialiser
/// resolves to one (`let l = env.ledger(); ... l.timestamp()`), up to
/// [`MAX_BINDINGS_HOPS`] hops.
fn is_ledger_receiver<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    resolve_to_ledger_call(cx, expr, MAX_BINDINGS_HOPS)
}

/// Returns `true` when `ty` is exactly `soroban_sdk::Env` (after peeling
/// references). A def-path match via the crate's shared helpers replaces the
/// old substring test on the printed type, which any user `EnvWrapper` or
/// locally named `Environment` struct satisfied (#795).
fn is_soroban_env<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    ty_adt_def(cx.typeck_results().expr_ty(expr).peel_refs())
        .is_some_and(|adt_def| matches_any_path(cx, adt_def.did(), SOROBAN_ENV_TYPES))
}

/// Follows the chain of call results backwards through simple `let` bindings:
/// the literal shape and every binding hop whose initialiser also resolves,
/// up to `hops` hops. `let l = env.ledger(); l.timestamp()` is one hop.
fn resolve_to_ledger_call<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    hops: usize,
) -> bool {
    match expr.kind {
        ExprKind::MethodCall(path, receiver, args, _) => {
            path.ident.as_str() == "ledger" && args.is_empty() && is_soroban_env(cx, receiver)
        }
        ExprKind::Path(QPath::Resolved(None, _)) => {
            if hops == 0 {
                return false;
            }
            if let Some(init) = let_initializer(cx, expr) {
                return resolve_to_ledger_call(cx, init, hops - 1);
            }
            false
        }
        _ => false,
    }
}

/// The initialiser of the `let` statement that binds `expr`'s local path, or
/// `None` when the path does not name a simple `let`-bound local. The `let`
/// statement may sit anywhere inside the enclosing body's block tree (only
/// nested blocks of the body itself are walked, not nested closure bodies).
fn let_initializer<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    let ExprKind::Path(QPath::Resolved(None, path)) = expr.kind else {
        return None;
    };
    let Res::Local(local_id) = path.res else {
        return None;
    };
    let owner = cx.tcx.hir_enclosing_body_owner(expr.hir_id);
    let body_id = cx.tcx.hir_node_by_def_id(owner).body_id()?;
    let body = cx.tcx.hir_body(body_id);
    find_let_initializer(body.value, local_id)
}

/// Walks the block tree of `expr` looking for the `let` statement whose
/// single binding pattern shares `local_id`, returning its initialiser.
fn find_let_initializer<'tcx>(expr: &'tcx Expr<'tcx>, local_id: HirId) -> Option<&'tcx Expr<'tcx>> {
    let mut stack = vec![expr];
    while let Some(e) = stack.pop() {
        let ExprKind::Block(block, _) = e.kind else {
            continue;
        };
        for stmt in block.stmts {
            if let rustc_hir::StmtKind::Let(let_stmt) = stmt.kind {
                if let PatKind::Binding(_, binding_id, _, None) = let_stmt.pat.kind
                    && binding_id == local_id
                {
                    return let_stmt.init;
                }
                // A nested initialiser's block tree is still part of this
                // body's walk.
                if let Some(init) = let_stmt.init {
                    stack.push(init);
                }
            }
        }
        if let Some(tail) = block.expr {
            stack.push(tail);
        }
    }
    None
}
