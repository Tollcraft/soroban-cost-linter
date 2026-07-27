#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg};
use clippy_utils::get_enclosing_loop_or_multi_call_closure;
use clippy_utils::source::snippet_opt;
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_hir::intravisit::{Visitor, walk_expr};
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
    SymbolOperations,
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
    LintMetadata {
        lint: SYMBOL_NEW_FOR_SHORT_LITERAL,
        category: LintCategory::SymbolOperations,
    },
    LintMetadata {
        lint: BLIND_STORAGE_WRITE,
        category: LintCategory::StorageOperations,
    },
];

#[unsafe(no_mangle)]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut LintStore) {
    lint_store.register_lints(&[
        SOROBAN_STORAGE_IN_LOOP,
        REDUNDANT_ENV_CLONE,
        UNNECESSARY_HOST_FUNCTION_CALL,
        HOST_IN_LOOP,
        SYMBOL_NEW_FOR_SHORT_LITERAL,
        BLIND_STORAGE_WRITE,
    ]);
    lint_store.register_late_pass(|_| Box::new(SorobanStorageInLoop));
    lint_store.register_late_pass(|_| Box::new(RedundantEnvClone));
    lint_store.register_late_pass(|_| Box::new(UnnecessaryHostFunctionCall));
    lint_store.register_late_pass(|_| Box::new(HostInLoop));
    lint_store.register_late_pass(|_| Box::new(SymbolNewForShortLiteral));
    lint_store.register_late_pass(|_| Box::new(BlindStorageWrite));
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

// =======================================================================
// symbol_new_for_short_literal — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub SYMBOL_NEW_FOR_SHORT_LITERAL,
    Warn,
    "Symbol::new used with a short literal that could use symbol_short! macro"
}
pub struct SymbolNewForShortLiteral;
rustc_session::impl_lint_pass!(SymbolNewForShortLiteral => [SYMBOL_NEW_FOR_SHORT_LITERAL]);

impl<'tcx> LateLintPass<'tcx> for SymbolNewForShortLiteral {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        // Check for Symbol::new(&env, "literal") calls
        if let hir::ExprKind::Call(callee, args) = expr.kind
            && args.len() == 2
            && let hir::ExprKind::Path(ref qpath) = callee.kind
            && let Some(def_id) = cx.qpath_res(qpath, callee.hir_id).opt_def_id()
            && match_soroban_def_path(cx, def_id, &["soroban_sdk", "Symbol", "new"])
        {
            // Check if the second argument is a string literal
            if let hir::ExprKind::Lit(lit) = args[1].kind
                && let LitKind::Str(symbol, _) = lit.node
            {
                let s = symbol.as_str();
                if is_valid_short_symbol(s) {
                    // Check if there's a valid suggestion
                    if let Some(snippet) = snippet_opt(cx, args[1].span) {
                        let suggestion = format!("symbol_short!({})", snippet);
                        span_lint_and_sugg(
                            cx,
                            SYMBOL_NEW_FOR_SHORT_LITERAL,
                            expr.span,
                            "Symbol::new called with a short literal that could use symbol_short! macro",
                            "use symbol_short! macro for compile-time symbol creation",
                            suggestion,
                            Applicability::MachineApplicable,
                        );
                    } else {
                        span_lint_and_help(
                            cx,
                            SYMBOL_NEW_FOR_SHORT_LITERAL,
                            expr.span,
                            "Symbol::new called with a short literal that could use symbol_short! macro",
                            None,
                            "use symbol_short! macro for compile-time symbol creation",
                        );
                    }
                }
            }
        }
    }
}

/// Check if a string is a valid short symbol (<= 9 chars, only a-zA-Z0-9_)
fn is_valid_short_symbol(s: &str) -> bool {
    if s.len() > 9 || s.is_empty() {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// =======================================================================
// blind_storage_write — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub BLIND_STORAGE_WRITE,
    Warn,
    "storage write without a preceding read on the same key"
}
pub struct BlindStorageWrite;
rustc_session::impl_lint_pass!(BlindStorageWrite => [BLIND_STORAGE_WRITE]);

/// Identifies which Soroban storage bucket a method call is targeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BlindStorageBucket {
    Instance,
    Persistent,
    Temporary,
}

impl<'tcx> LateLintPass<'tcx> for BlindStorageWrite {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _fn_kind: hir::intravisit::FnKind<'tcx>,
        _fn_decl: &'tcx hir::FnDecl<'tcx>,
        body: &'tcx hir::Body<'tcx>,
        _span: rustc_span::Span,
        _fn_def_id: hir::def_id::LocalDefId,
    ) {
        let mut visitor = BlindStorageVisitor {
            cx,
            seen_reads: std::collections::HashSet::new(),
        };
        visitor.visit_body(body);
    }
}

/// Walks a function body linearly, recording the (storage-bucket, key) pairs
/// that have been read, and emitting a `blind_storage_write` warning whenever
/// a storage `.set()` call targets a key that has never been read anywhere in
/// the same function.
///
/// A HIR-level pre-order walk gives us the source-order semantics we want: a
/// read that appears textually before the write "authorises" that write. We
/// intentionally do not perform control-flow sensitive analysis — over the
/// whole function is the conservative, low-false-positive choice for a
/// cost-shape lint.
struct BlindStorageVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    seen_reads: std::collections::HashSet<(BlindStorageBucket, String)>,
}

impl<'a, 'tcx> BlindStorageVisitor<'a, 'tcx> {
    /// Returns the storage bucket targeted by the receiver of `expr`, if
    /// `expr` is a method call on `Instance`, `Persistent`, or `Temporary`.
    fn detect_storage_bucket(&self, expr: &hir::Expr<'tcx>) -> Option<BlindStorageBucket> {
        if let hir::ExprKind::MethodCall(_, receiver, _, _) = expr.kind {
            let receiver_ty = self.cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();
            if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                let did = adt_def.did();
                if match_soroban_def_path(self.cx, did, &["soroban_sdk", "storage", "Instance"]) {
                    return Some(BlindStorageBucket::Instance);
                }
                if match_soroban_def_path(self.cx, did, &["soroban_sdk", "storage", "Persistent"]) {
                    return Some(BlindStorageBucket::Persistent);
                }
                if match_soroban_def_path(self.cx, did, &["soroban_sdk", "storage", "Temporary"]) {
                    return Some(BlindStorageBucket::Temporary);
                }
            }
        }
        None
    }
}

impl<'a, 'tcx> Visitor<'tcx> for BlindStorageVisitor<'a, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, _receiver, args, _span) = expr.kind
            && let Some(bucket) = self.detect_storage_bucket(expr)
            && let Some(key_arg) = args.first()
        {
            let method = path_segment.ident.name.as_str();

            match method {
                "set" => {
                    // Only flag blind writes when we have a usable textual key
                    // (so the comparison against earlier reads is meaningful).
                    // Complex key expressions are intentionally skipped.
                    if let Some(key_text) = snippet_opt(self.cx, key_arg.span)
                        && !self.seen_reads.contains(&(bucket, key_text.clone()))
                    {
                        span_lint_and_help(
                            self.cx,
                            BLIND_STORAGE_WRITE,
                            expr.span,
                            "blind storage write (no preceding read on this key)",
                            None,
                            "read the existing value first with `.get()`, `.has()`, or `.remove()` so the contract does not silently overwrite or collide with existing entries",
                        );
                    }
                }
                "get" | "try_get" | "has" | "remove" | "update" => {
                    // Any non-write interaction with the same key counts as a
                    // "read" that authorises a later write on the same key.
                    if let Some(key_text) = snippet_opt(self.cx, key_arg.span) {
                        self.seen_reads.insert((bucket, key_text));
                    }
                }
                _ => {}
            }
        }

        walk_expr(self, expr);
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
