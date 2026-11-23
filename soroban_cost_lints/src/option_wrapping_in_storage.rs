use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::is_in_test;
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
            let value_ty = cx.typeck_results().expr_ty(value_expr).peel_refs();
            // Check if the value type is Option<T> at the top level
            if is_option_type(cx, value_ty) {
                if is_in_test(cx.tcx, expr.hir_id) {
                    return;
                }
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

/// Returns `true` if `expr` is the receiver of a storage `.set()` call,
/// verified by matching against canonical [`SOROBAN_STORAGE_TYPES`] definition
/// paths after peeling any references.
fn is_storage_set_receiver<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr).peel_refs();
    is_storage_type(cx, ty)
}

/// Returns `true` if `ty` is one of the Soroban storage accessor types
/// by checking its canonical definition path against [`SOROBAN_STORAGE_TYPES`].
/// This avoids false positives on user-defined types whose names happen to
/// contain "Instance", "Persistent", "Temporary", or "Storage".
fn is_storage_type<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    ty_adt_def(ty).is_some_and(|adt_def| matches_any_path(cx, adt_def.did(), SOROBAN_STORAGE_TYPES))
}

/// Returns `true` if `ty` is `Option<T>` (not a struct that merely contains
/// an Option field). References are peeled before the check so that `&Option<T>`
/// and `&&Option<T>` are handled the same as `Option<T>`.
fn is_option_type(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    if let ty::Adt(adt, _) = ty.peel_refs().kind() {
        cx.tcx.is_diagnostic_item(sym::Option, adt.did())
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::SOROBAN_STORAGE_TYPES;

    /// Every SOROBAN_STORAGE_TYPES path starts with `soroban_sdk` so that
    /// `match_soroban_def_path` can verify the crate root when matching.
    #[test]
    fn storage_type_paths_start_with_soroban_sdk() {
        for path in SOROBAN_STORAGE_TYPES {
            assert_eq!(
                path[0], "soroban_sdk",
                "SOROBAN_STORAGE_TYPES entry {:?} must start with 'soroban_sdk'",
                path
            );
        }
    }

    /// All four canonical Soroban storage accessor types are present.
    #[test]
    fn storage_types_cover_all_four_accessors() {
        let last_segments: Vec<&str> = SOROBAN_STORAGE_TYPES
            .iter()
            .map(|path| *path.last().unwrap())
            .collect();
        for name in &["Storage", "Instance", "Persistent", "Temporary"] {
            assert!(
                last_segments.contains(name),
                "SOROBAN_STORAGE_TYPES must include '{name}'"
            );
        }
    }

    /// User-defined type names that happen to contain storage substrings must
    /// NOT be listed in SOROBAN_STORAGE_TYPES — they differ in def-path and
    /// the canonical path match rejects them.
    #[test]
    fn false_positive_names_are_not_in_storage_types() {
        let false_positive_names = [
            "InstanceConfiguration",
            "PersistentBuffer",
            "StorageManager",
            "TemporaryData",
        ];
        let last_segments: Vec<&str> = SOROBAN_STORAGE_TYPES
            .iter()
            .map(|path| *path.last().unwrap())
            .collect();
        for name in &false_positive_names {
            assert!(
                !last_segments.contains(name),
                "'{name}' should not be in SOROBAN_STORAGE_TYPES"
            );
        }
    }
}
