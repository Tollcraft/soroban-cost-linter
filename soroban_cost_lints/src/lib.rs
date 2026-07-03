#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;

use rustc_lint::{LateContext, LateLintPass};
use rustc_hir as hir;
use clippy_utils::get_enclosing_loop_or_multi_call_closure;
use clippy_utils::diagnostics::span_lint_and_help;

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Detects storage operations (reads/writes) placed inside loop bodies.
    ///
    /// ### Why is this bad?
    /// Storage operations in Soroban are the most expensive resource. Placing them in a loop
    /// wastes CPU instructions and ledger write/read throughput.
    ///
    /// ### Example
    /// ```rust
    /// for item in items {
    ///     env.storage().instance().set(&item, &1);
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// // Accumulate mutations in memory and write once or optimize storage access.
    /// ```
    pub SOROBAN_STORAGE_IN_LOOP,
    Warn,
    "storage operations inside a loop"
}

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

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
