use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::ops::is_inside_loop;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::LEDGER_CONTEXT_READ_IN_LOOP;

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
            if is_inside_loop(cx, expr) {
                let help = format!(
                    "ledger context values ({method_name}) are invariant during a single \
                     invocation; hoist this read outside the loop to avoid repeated host calls"
                );
                span_lint_and_help(
                    cx,
                    LEDGER_CONTEXT_READ_IN_LOOP,
                    expr.span,
                    &format!(
                        "reading ledger context `{method_name}` inside a loop — the value \
                         cannot change during this invocation"
                    ),
                    None,
                    &help,
                );
            }
        }
    }
}

/// Returns `true` if `expr` is the result of calling `env.ledger()`.
fn is_ledger_receiver(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Check for <env>.ledger() method call
    if let rustc_hir::ExprKind::MethodCall(path, receiver, args, _) = expr.kind {
        if path.ident.as_str() == "ledger" && args.is_empty() {
            // The receiver of ledger() should be an Env-like type
            let ty = cx.typeck_results().expr_ty(receiver);
            let ty_str = format!("{:?}", ty);
            return ty_str.contains("Env");
        }
    }
    false
}
