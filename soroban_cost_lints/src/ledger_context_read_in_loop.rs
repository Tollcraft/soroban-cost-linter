use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::get_enclosing_loop_or_multi_call_closure;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{LEDGER_CONTEXT_READ_IN_LOOP, match_soroban_def_path, ty_adt_def};

/// Canonical Soroban ledger accessor type path.
const LEDGER_TYPE_PATH: &[&str] = &["soroban_sdk", "ledger", "Ledger"];

declare_lint_pass!(LedgerContextReadInLoop => [LEDGER_CONTEXT_READ_IN_LOOP]);

const LEDGER_READ_METHODS: &[&str] = &["sequence", "timestamp", "network_id", "protocol_version"];

impl<'tcx> LateLintPass<'tcx> for LedgerContextReadInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Look for method calls: <ledger>.<accessor>()
        if let rustc_hir::ExprKind::MethodCall(path, receiver, _args, _) = expr.kind {
            let method_name = path.ident.as_str();
            // Check if this is a ledger context accessor
            if !LEDGER_READ_METHODS.contains(&method_name) {
                return;
            }
            // The receiver must be a ledger() call on Env
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

/// Returns `true` if `expr` resolves to a Soroban `Ledger` accessor, handling
/// both direct chained calls (`env.ledger().sequence()`) and local bindings
/// (`let ledger = env.ledger(); ledger.sequence()`).
///
/// Type-based detection: check whether the expression's type is
/// `soroban_sdk::ledger::Ledger` by its canonical definition path rather than
/// by inspecting the expression shape. This catches both forms uniformly.
fn is_ledger_receiver<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr).peel_refs();
    ty_adt_def(ty)
        .is_some_and(|adt_def| match_soroban_def_path(cx, adt_def.did(), LEDGER_TYPE_PATH))
}
