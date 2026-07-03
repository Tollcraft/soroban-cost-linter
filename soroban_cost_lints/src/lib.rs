#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

use rustc_lint::{LateContext, LateLintPass, LintStore};
use rustc_hir as hir;
use clippy_utils::get_enclosing_loop_or_multi_call_closure;
use clippy_utils::diagnostics::span_lint_and_help;

dylint_linting::dylint_library!();

#[no_mangle]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut LintStore) {
    lint_store.register_lints(&[
        SOROBAN_STORAGE_IN_LOOP,
        REDUNDANT_ENV_CLONE,
        UNNECESSARY_HOST_FUNCTION_CALL,
    ]);
    lint_store.register_late_pass(|_| Box::new(SorobanStorageInLoop));
    lint_store.register_late_pass(|_| Box::new(RedundantEnvClone));
    lint_store.register_late_pass(|_| Box::new(UnnecessaryHostFunctionCall));
}

rustc_lint::declare_lint! {
    /// ### What it does
    /// Detects storage operations (reads/writes) placed inside loop bodies.
    pub SOROBAN_STORAGE_IN_LOOP,
    Warn,
    "storage operations inside a loop"
}
pub struct SorobanStorageInLoop;
rustc_lint::impl_lint_pass!(SorobanStorageInLoop => [SOROBAN_STORAGE_IN_LOOP]);

impl<'tcx> LateLintPass<'tcx> for SorobanStorageInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();
            
            let is_storage_access = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                let path = cx.tcx.def_path_str(adt_def.did());
                path == "soroban_sdk::storage::Storage"
                    || path.ends_with("::soroban_sdk::storage::Storage")
                    || path == "soroban_sdk::storage::Instance"
                    || path.ends_with("::soroban_sdk::storage::Instance")
                    || path == "soroban_sdk::storage::Persistent"
                    || path.ends_with("::soroban_sdk::storage::Persistent")
                    || path == "soroban_sdk::storage::Temporary"
                    || path.ends_with("::soroban_sdk::storage::Temporary")
                    || ((path == "soroban_sdk::Env" || path.ends_with("::soroban_sdk::Env")) && path_segment.ident.name.as_str() == "storage")
            } else {
                false
            };

            if is_storage_access {
                if let Some(enclosing_expr) = get_enclosing_loop_or_multi_call_closure(cx, expr) {
                    if let hir::ExprKind::Loop(..) = enclosing_expr.kind {
                        span_lint_and_help(
                            cx,
                            SOROBAN_STORAGE_IN_LOOP,
                            expr.span,
                            "storage operation inside a loop",
                            None,
                            "move storage operations out of the loop or accumulate mutations in memory first",
                        );
                    }
                }
            }
        }
    }
}

rustc_lint::declare_lint! {
    /// ### What it does
    /// Detects unnecessary `.clone()` calls on the Soroban `Env` object.
    pub REDUNDANT_ENV_CLONE,
    Warn,
    "redundant clone on Env object"
}
pub struct RedundantEnvClone;
rustc_lint::impl_lint_pass!(RedundantEnvClone => [REDUNDANT_ENV_CLONE]);

impl<'tcx> LateLintPass<'tcx> for RedundantEnvClone {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            if path_segment.ident.name.as_str() == "clone" {
                let receiver_ty = cx.typeck_results().expr_ty(receiver);
                let peeled_ty = receiver_ty.peel_refs();
                
                let is_env = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                    let path = cx.tcx.def_path_str(adt_def.did());
                    path == "soroban_sdk::Env" || path.ends_with("::soroban_sdk::Env")
                } else {
                    false
                };

                if is_env {
                    span_lint_and_help(
                        cx,
                        REDUNDANT_ENV_CLONE,
                        expr.span,
                        "redundant clone on Env object",
                        None,
                        "pass Env by reference or value instead of cloning",
                    );
                }
            }
        }
    }
}

rustc_lint::declare_lint! {
    /// ### What it does
    /// Identifies redundant calls to host functions inside loop bodies.
    pub UNNECESSARY_HOST_FUNCTION_CALL,
    Warn,
    "unnecessary host function call inside loop"
}
pub struct UnnecessaryHostFunctionCall;
rustc_lint::impl_lint_pass!(UnnecessaryHostFunctionCall => [UNNECESSARY_HOST_FUNCTION_CALL]);

impl<'tcx> LateLintPass<'tcx> for UnnecessaryHostFunctionCall {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(_path_segment, receiver, _args, _span) = expr.kind {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();
            
            let is_host_function = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                let path = cx.tcx.def_path_str(adt_def.did());
                path == "soroban_sdk::ledger::Ledger" || path.ends_with("::soroban_sdk::ledger::Ledger")
            } else {
                false
            };

            if is_host_function {
                if let Some(enclosing_expr) = get_enclosing_loop_or_multi_call_closure(cx, expr) {
                    if let hir::ExprKind::Loop(..) = enclosing_expr.kind {
                        span_lint_and_help(
                            cx,
                            UNNECESSARY_HOST_FUNCTION_CALL,
                            expr.span,
                            "unnecessary host function call inside loop",
                            None,
                            "call this function outside the loop and reuse the result",
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
