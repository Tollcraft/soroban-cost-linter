use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

use crate::OPTION_WRAPPING_IN_STORAGE;

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
                cx.span_lint(
                    OPTION_WRAPPING_IN_STORAGE,
                    expr.span,
                    "storage write stores an `Option<T>` — storage already models absence, so this creates a redundant three-state model (missing / Some / None)",
                    |diag| {
                        diag.help(help);
                    },
                );
            }
        }
    }
}

/// Returns `true` if `expr` is the receiver of a storage `.set()` call
/// (i.e. the receiver is one of Instance, Persistent, or Temporary).
fn is_storage_set_receiver(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr);
    is_storage_type(ty)
}

/// Returns `true` if `ty` is one of the Soroban storage container types
/// (Storage, Instance, Persistent, Temporary).
fn is_storage_type(ty: Ty<'_>) -> bool {
    let ty_str = format!("{:?}", ty);
    ty_str.contains("Instance")
        || ty_str.contains("Persistent")
        || ty_str.contains("Temporary")
        || ty_str.contains("Storage")
}

/// Returns `true` if `ty` is exactly `Option<T>` (not a struct that merely
/// contains an Option field).
fn is_option_type(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    match ty.kind() {
        ty::Adt(adt, args) if cx.tcx.is_diagnostic_item(sym::Option, adt.did()) => {
            // Verify we have a single generic argument (Option<T> has one)
            args.count() == 1
        }
        _ => false,
    }
}
