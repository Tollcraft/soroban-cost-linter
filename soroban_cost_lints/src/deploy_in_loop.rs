//! `deploy_in_loop` — flags contract deployment calls inside a loop.
//!
//! `env.deployer().deploy(...)` and `deploy_v2(...)` inside a loop instantiate a
//! contract and Wasm bytecode per iteration, one of the most expensive host
//! operations exposed by the Soroban SDK.

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{DEPLOY_IN_LOOP, enclosing_loop, is_type_match};

declare_lint_pass!(DeployInLoop => [DEPLOY_IN_LOOP]);

pub(crate) const SOROBAN_DEPLOYER_TYPES: &[&[&str]] = &[
    &["soroban_sdk", "deploy", "Deployer"],
    &["soroban_sdk", "deploy", "DeployerWithAddress"],
    &["soroban_sdk", "deploy", "DeployerWithAsset"],
];

const DEPLOY_IN_LOOP_MSG: &str = "contract deployment inside a loop";
const DEPLOY_IN_LOOP_HELP: &str =
    "deploy once and store the contract id, or move the deployment out of the loop";

impl<'tcx> LateLintPass<'tcx> for DeployInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            let method_name = path_segment.ident.name.as_str();
            if method_name != "deploy" && method_name != "deploy_v2" {
                return;
            }

            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            if !is_type_match(cx, receiver_ty, SOROBAN_DEPLOYER_TYPES) {
                return;
            }

            if enclosing_loop(cx, expr).is_some() {
                span_lint_and_help(
                    cx,
                    DEPLOY_IN_LOOP,
                    expr.span,
                    DEPLOY_IN_LOOP_MSG,
                    None,
                    DEPLOY_IN_LOOP_HELP,
                );
            }
        }
    }
}
