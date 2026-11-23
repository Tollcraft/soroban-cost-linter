//! `token_transfer_in_loop` — flags token transfer calls made through contract clients inside loops.
//!
//! A token transfer is a cross-contract invocation plus at least two storage writes,
//! making it one of the most expensive operations a contract can perform. Repeating it
//! inside a loop multiplies that cost by a caller-influenced factor.

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::get_enclosing_loop_or_multi_call_closure;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{TOKEN_TRANSFER_IN_LOOP, enclosing_loop};

declare_lint_pass!(TokenTransferInLoop => [TOKEN_TRANSFER_IN_LOOP]);

const TOKEN_TRANSFER_IN_LOOP_MSG: &str = "token transfer inside a loop";
const TOKEN_TRANSFER_IN_LOOP_HELP: &str = "batch the transfer, or switch to a claim pattern where recipients pull instead of the contract pushing";

impl<'tcx> LateLintPass<'tcx> for TokenTransferInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind
            && matches!(
                path_segment.ident.name.as_str(),
                "transfer" | "transfer_from"
            )
        {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_client = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                let item_name = cx.tcx.item_name(adt_def.did());
                item_name.as_str().ends_with("Client")
            } else {
                false
            };

            if is_client
                && (enclosing_loop(cx, expr).is_some()
                    || get_enclosing_loop_or_multi_call_closure(cx, expr).is_some())
            {
                span_lint_and_help(
                    cx,
                    TOKEN_TRANSFER_IN_LOOP,
                    expr.span,
                    TOKEN_TRANSFER_IN_LOOP_MSG,
                    None,
                    TOKEN_TRANSFER_IN_LOOP_HELP,
                );
            }
        }
    }
}
