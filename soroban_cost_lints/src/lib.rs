#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::get_enclosing_loop_or_multi_call_closure;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintStore};
use rustc_span::def_id::DefId;

dylint_linting::dylint_library!();

fn match_soroban_def_path<'tcx>(cx: &LateContext<'tcx>, def_id: DefId, segments: &[&str]) -> bool {
    let full = cx.tcx.def_path_str(def_id);
    let suffix: String = segments.join("::");
    full.ends_with(&suffix)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintCategory {
    StorageOperations,
    Compute,
    Memory,
    EntryLifecycle,
}

pub struct LintMetadata {
    pub lint: &'static rustc_lint::Lint,
    pub category: LintCategory,
}

pub const LINT_METADATA: &[LintMetadata] = &[
    LintMetadata {
        lint: SOROBAN_STORAGE_IN_LOOP,
        category: LintCategory::StorageOperations,
    },
    LintMetadata {
        lint: SOROBAN_REDUNDANT_STORAGE_READ,
        category: LintCategory::StorageOperations,
    },
    LintMetadata {
        lint: REDUNDANT_ENV_CLONE,
        category: LintCategory::Memory,
    },
    LintMetadata {
        lint: UNNECESSARY_HOST_FUNCTION_CALL,
        category: LintCategory::Compute,
    },
    LintMetadata {
        lint: HOST_IN_LOOP,
        category: LintCategory::Compute,
    },
];

#[unsafe(no_mangle)]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut LintStore) {
    lint_store.register_lints(&[
        SOROBAN_STORAGE_IN_LOOP,
        SOROBAN_REDUNDANT_STORAGE_READ,
        REDUNDANT_ENV_CLONE,
        UNNECESSARY_HOST_FUNCTION_CALL,
        HOST_IN_LOOP,
    ]);
    lint_store.register_late_pass(|_| Box::new(SorobanStorageInLoop));
    lint_store.register_late_pass(|_| Box::new(SorobanRedundantStorageRead));
    lint_store.register_late_pass(|_| Box::new(RedundantEnvClone));
    lint_store.register_late_pass(|_| Box::new(UnnecessaryHostFunctionCall));
    lint_store.register_late_pass(|_| Box::new(HostInLoop));
}

rustc_session::declare_lint! {
    pub SOROBAN_STORAGE_IN_LOOP,
    Warn,
    "storage operations inside a loop"
}
pub struct SorobanStorageInLoop;
rustc_session::impl_lint_pass!(SorobanStorageInLoop => [SOROBAN_STORAGE_IN_LOOP]);

impl<'tcx> LateLintPass<'tcx> for SorobanStorageInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_storage_access = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                let did = adt_def.did();
                match_soroban_def_path(cx, did, &["soroban_sdk", "storage", "Storage"])
                    || match_soroban_def_path(cx, did, &["soroban_sdk", "storage", "Instance"])
                    || match_soroban_def_path(cx, did, &["soroban_sdk", "storage", "Persistent"])
                    || match_soroban_def_path(cx, did, &["soroban_sdk", "storage", "Temporary"])
                    || (match_soroban_def_path(cx, did, &["soroban_sdk", "Env"])
                        && path_segment.ident.name.as_str() == "storage")
            } else {
                false
            };

            if is_storage_access
                && let Some(enclosing_expr) = get_enclosing_loop_or_multi_call_closure(cx, expr)
                && let hir::ExprKind::Loop(..) = enclosing_expr.kind
            {
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

rustc_session::declare_lint! {
    pub SOROBAN_REDUNDANT_STORAGE_READ,
    Warn,
    "multiple sequential reads of the same storage key without modification"
}
pub struct SorobanRedundantStorageRead;
rustc_session::impl_lint_pass!(SorobanRedundantStorageRead => [SOROBAN_REDUNDANT_STORAGE_READ]);

impl SorobanRedundantStorageRead {
    fn is_storage_type<'tcx>(cx: &LateContext<'tcx>, ty: rustc_middle::ty::Ty<'tcx>) -> Option<DefId> {
        let peeled = ty.peel_refs();
        if let rustc_middle::ty::Adt(adt_def, _) = peeled.kind() {
            let did = adt_def.did();
            if match_soroban_def_path(cx, did, &["soroban_sdk", "storage", "Instance"])
                || match_soroban_def_path(cx, did, &["soroban_sdk", "storage", "Persistent"])
                || match_soroban_def_path(cx, did, &["soroban_sdk", "storage", "Temporary"])
            {
                return Some(did);
            }
        }
        None
    }

    fn extract_storage_op<'tcx>(
        cx: &LateContext<'tcx>,
        expr: &'tcx hir::Expr<'tcx>,
    ) -> Option<StorageOp> {
        if let hir::ExprKind::MethodCall(path_segment, receiver, args, _span) = expr.kind {
            let method_name = path_segment.ident.name.as_str();
            if method_name != "get" && method_name != "has" && method_name != "set" {
                return None;
            }

            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let storage_def_id = Self::is_storage_type(cx, receiver_ty)?;

            if method_name == "set" {
                return Some(StorageOp::Write);
            }

            // For get/has, extract the key argument and get its source text
            let key_arg = args.first()?;
            let key_inner = if let hir::ExprKind::AddrOf(_, _, inner) = key_arg.kind {
                inner
            } else {
                key_arg
            };

            let key_text = cx
                .sess()
                .source_map()
                .span_to_snippet(key_inner.span)
                .ok()?;

            Some(StorageOp::Read {
                storage_def_id,
                key_text,
            })
        } else {
            None
        }
    }
}

enum StorageOp {
    Read {
        storage_def_id: DefId,
        key_text: String,
    },
    Write,
}

impl<'tcx> LateLintPass<'tcx> for SorobanRedundantStorageRead {
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx hir::Block<'tcx>) {
        let mut last_read: Option<(DefId, String)> = None;

        // Iterate over top-level expressions from statements and optional tail expr
        let exprs = block
            .stmts
            .iter()
            .filter_map(|stmt| match stmt.kind {
                hir::StmtKind::Let(hir::LetStmt { init: Some(init), .. }) => Some(init),
                hir::StmtKind::Expr(expr) | hir::StmtKind::Semi(expr) => Some(expr),
                _ => None,
            })
            .chain(block.expr);

        for expr in exprs {
            if let Some(op) = SorobanRedundantStorageRead::extract_storage_op(cx, expr) {
                match op {
                    StorageOp::Read {
                        storage_def_id,
                        key_text,
                    } => {
                        if let Some((last_def_id, ref last_key)) = last_read {
                            if last_def_id == storage_def_id && *last_key == key_text {
                                span_lint_and_help(
                                    cx,
                                    SOROBAN_REDUNDANT_STORAGE_READ,
                                    expr.span,
                                    "redundant storage read: this key was already read without modification",
                                    None,
                                    "store the value from the first read and reuse it instead of reading again",
                                );
                            }
                        }
                        last_read = Some((storage_def_id, key_text));
                    }
                    StorageOp::Write => {
                        last_read = None;
                    }
                }
            }
        }
    }
}

rustc_session::declare_lint! {
    pub REDUNDANT_ENV_CLONE,
    Warn,
    "redundant clone on Env object"
}
pub struct RedundantEnvClone;
rustc_session::impl_lint_pass!(RedundantEnvClone => [REDUNDANT_ENV_CLONE]);

impl<'tcx> LateLintPass<'tcx> for RedundantEnvClone {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind
            && path_segment.ident.name.as_str() == "clone"
        {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_env = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "Env"])
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

rustc_session::declare_lint! {
    pub UNNECESSARY_HOST_FUNCTION_CALL,
    Warn,
    "unnecessary host function call inside loop"
}
pub struct UnnecessaryHostFunctionCall;
rustc_session::impl_lint_pass!(UnnecessaryHostFunctionCall => [UNNECESSARY_HOST_FUNCTION_CALL]);

rustc_session::declare_lint! {
    pub HOST_IN_LOOP,
    Warn,
    "use of Host object inside a loop"
}
pub struct HostInLoop;
rustc_session::impl_lint_pass!(HostInLoop => [HOST_IN_LOOP]);

impl<'tcx> LateLintPass<'tcx> for UnnecessaryHostFunctionCall {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(_path_segment, receiver, _args, _span) = expr.kind {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_host_function = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "ledger", "Ledger"])
            } else {
                false
            };

            if is_host_function
                && let Some(enclosing_expr) = get_enclosing_loop_or_multi_call_closure(cx, expr)
                && let hir::ExprKind::Loop(..) = enclosing_expr.kind
            {
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

impl<'tcx> LateLintPass<'tcx> for HostInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(_path_segment, receiver, _args, _span) = expr.kind {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_host = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                match_soroban_def_path(cx, adt_def.did(), &["host", "Host"])
            } else {
                false
            };

            if is_host
                && let Some(enclosing_expr) = get_enclosing_loop_or_multi_call_closure(cx, expr)
                && let hir::ExprKind::Loop(..) = enclosing_expr.kind
            {
                span_lint_and_help(
                    cx,
                    HOST_IN_LOOP,
                    expr.span,
                    "use of Host object inside a loop",
                    None,
                    "consider moving the Host usage outside the loop if possible",
                );
            }
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
