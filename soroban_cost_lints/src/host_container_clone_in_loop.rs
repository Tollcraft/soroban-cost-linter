//! `host_container_clone_in_loop` — flags cloning Soroban SDK host containers inside a loop.
//!
//! Cloning a `soroban_sdk::Vec`, `Map`, `Bytes`, or `String` inside a loop deep-copies
//! the host object on every iteration, causing the host to serialize and duplicate the whole
//! payload N times.
//!
//! # Deliberate difference from `RedundantEnvClone`
//!
//! Unlike `RedundantEnvClone`, which inspects cheap `Env` handles and bails out when the receiver
//! is a reference or when the local binding is still used later (`local_used_after_expr`),
//! cloning a host container always performs a real copy regardless of whether the original
//! is live or whether the receiver is a reference. Therefore, those guards are deliberately
//! omitted here.

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{HOST_CONTAINER_CLONE_IN_LOOP, SOROBAN_CONTAINER_TYPES, enclosing_loop, is_type_match};

declare_lint_pass!(HostContainerCloneInLoop => [HOST_CONTAINER_CLONE_IN_LOOP]);

const HOST_CONTAINER_CLONE_IN_LOOP_MSG: &str = "cloning a host container inside a loop";
const HOST_CONTAINER_CLONE_IN_LOOP_HELP: &str =
    "borrow the container or move the clone before the loop";

const SOROBAN_STRING_TYPE: &[&str] = &["soroban_sdk", "String"];

impl<'tcx> LateLintPass<'tcx> for HostContainerCloneInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            if path_segment.ident.name.as_str() != "clone" {
                return;
            }

            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let target_types = [
                SOROBAN_CONTAINER_TYPES[0],
                SOROBAN_CONTAINER_TYPES[1],
                SOROBAN_CONTAINER_TYPES[2],
                SOROBAN_STRING_TYPE,
            ];
            if !is_type_match(cx, receiver_ty, &target_types) {
                return;
            }

            if enclosing_loop(cx, expr).is_some() {
                span_lint_and_help(
                    cx,
                    HOST_CONTAINER_CLONE_IN_LOOP,
                    expr.span,
                    HOST_CONTAINER_CLONE_IN_LOOP_MSG,
                    None,
                    HOST_CONTAINER_CLONE_IN_LOOP_HELP,
                );
            }
        }
    }
}
