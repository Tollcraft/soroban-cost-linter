//! `auths_read_in_loop` — flags calling `env.auths()` inside a loop.
//!
//! `env.auths()` returns the authorized-address list for the current invocation.
//! The host materialises and copies the whole authorization tree per call,
//! yet the set is fixed before the contract starts executing. Calling it inside
//! a loop repeatedly pays this host cost on every iteration.

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{AUTHS_READ_IN_LOOP, enclosing_loop, match_soroban_def_path, ty_adt_def};

declare_lint_pass!(AuthsReadInLoop => [AUTHS_READ_IN_LOOP]);

const AUTHS_READ_IN_LOOP_MSG: &str = "calling env.auths() inside a loop";
const AUTHS_READ_IN_LOOP_HELP: &str = "bind env.auths() once before the loop and reuse it";

impl<'tcx> LateLintPass<'tcx> for AuthsReadInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            if path_segment.ident.name.as_str() != "auths" {
                return;
            }

            let receiver_ty = cx.typeck_results().expr_ty(receiver).peel_refs();
            let is_env = if let Some(adt_def) = ty_adt_def(receiver_ty) {
                match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "Env"])
            } else {
                false
            };

            if !is_env {
                return;
            }

            if enclosing_loop(cx, expr).is_some() {
                span_lint_and_help(
                    cx,
                    AUTHS_READ_IN_LOOP,
                    expr.span,
                    AUTHS_READ_IN_LOOP_MSG,
                    None,
                    AUTHS_READ_IN_LOOP_HELP,
                );
            }
        }
    }
}
