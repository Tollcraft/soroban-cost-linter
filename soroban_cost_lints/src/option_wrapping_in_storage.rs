use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

use crate::{OPTION_WRAPPING_IN_STORAGE, SOROBAN_STORAGE_TYPES, matches_any_path, ty_adt_def};

declare_lint_pass!(OptionWrappingInStorage => [OPTION_WRAPPING_IN_STORAGE]);

impl<'tcx> LateLintPass<'tcx> for OptionWrappingInStorage {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Look for method calls: <receiver>.set(&key, &value)
        if let rustc_hir::ExprKind::MethodCall(path, receiver, args, _) = expr.kind {
            let method_name = path.ident.as_str();
            if method_name != "set" {
                return;
            }
            // Need exactly two arguments (key and value)
            if args.len() != 2 {
                return;
            }
            // The receiver must be a storage type (Instance, Persistent, Temporary)
            if !is_storage_set_receiver(cx, receiver) {
                return;
            }
            // Get the type of the value argument (second arg)
            let value_expr = &args[1];
            let value_ty = cx.typeck_results().expr_ty(value_expr);
            // Check if the value type is Option<T> at the top level
            if is_option_type(cx, value_ty) {
                let help = "storage already models absence (missing key = None); \
                    store the inner type T directly and remove the key instead of wrapping in Option";
                span_lint_and_help(
                    cx,
                    OPTION_WRAPPING_IN_STORAGE,
                    expr.span,
                    "storage write stores an `Option<T>` — storage already models absence, so this creates a redundant three-state model (missing / Some / None)",
                    None,
                    help,
                );
            }
        }
    }
}

/// Returns `true` if `expr` is the receiver of a storage `.set()` call
/// (i.e. the receiver is one of Instance, Persistent, or Temporary).
fn is_storage_set_receiver<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    is_storage_type(cx, cx.typeck_results().expr_ty(expr))
}

/// Returns `true` if `ty` is one of the Soroban storage container types
/// (`soroban_sdk::storage::{Storage, Instance, Persistent, Temporary}`), after
/// peeling references so `&Persistent` behaves like `Persistent`. The def-path
/// match replaces the old Debug-printed substring test, which over-matched any
/// user type whose printed name contained one of those words (#796).
fn is_storage_type<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    ty_adt_def(ty.peel_refs())
        .is_some_and(|adt_def| matches_any_path(cx, adt_def.did(), SOROBAN_STORAGE_TYPES))
}

/// Returns `true` if `ty` is exactly `Option<T>` (not a struct that merely
/// contains an Option field).
fn is_option_type(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    match ty.kind() {
        ty::Adt(adt, args) if cx.tcx.is_diagnostic_item(sym::Option, adt.did()) => {
            // Verify we have a single generic argument (Option<T> has one)
            args.len() == 1
        }
        _ => false,
    }
}
