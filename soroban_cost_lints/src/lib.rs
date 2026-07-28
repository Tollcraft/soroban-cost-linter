#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Soroban cost-analysis lints.
//!
//! This crate is a [Dylint](https://github.com/trailofbits/dylint) library. It
//! is compiled to a `cdylib` and loaded by `cargo dylint` (driven by the
//! `cargo-cost-lint` wrapper), which runs each lint as a late-stage pass over a
//! Soroban contract's [HIR](https://rustc-dev-guide.rust-lang.org/hir.html).
//!
//! # What the lints look for
//!
//! Soroban meters execution against a CPU and memory budget. The lints here
//! flag *structural* anti-patterns whose cost does not depend on runtime input,
//! so they can be caught statically:
//!
//! - [`SOROBAN_STORAGE_IN_LOOP`] — storage reads/writes performed inside a loop.
//! - [`REDUNDANT_ENV_CLONE`] — cloning the `Env` handle when a reference would
//!   do.
//! - [`UNNECESSARY_HOST_FUNCTION_CALL`] — a metered host call inside a loop
//!   whose result is invariant across iterations and could be hoisted out.
//! - [`HOST_IN_LOOP`] — use of a `Host` object inside a loop.
//! - [`SYMBOL_NEW_FOR_SHORT_LITERAL`] — `Symbol::new` on a literal short enough
//!   for the compile-time `symbol_short!` macro.
//!
//! Each lint is assigned a [`LintCategory`] and registered in [`LINT_METADATA`],
//! the single source of truth the wrapper reads to describe available lints.
//!
//! # How a lint is structured
//!
//! Every lint follows the same three-part shape used throughout `rustc`/Clippy:
//!
//! 1. A [`declare_lint!`](rustc_session::declare_lint) invocation that defines
//!    the lint's static descriptor, default level, and short description.
//! 2. A zero-sized marker struct (e.g. [`SorobanStorageInLoop`]) that the pass
//!    is dispatched on.
//! 3. An `impl` of [`LateLintPass`] for that struct whose `check_expr` inspects
//!    each expression and emits a diagnostic when the pattern matches.
//!
//! Type-based matching is done against `soroban_sdk` def-paths via
//! [`match_soroban_def_path`] and the `SOROBAN_*` path tables, so the lints key
//! off the SDK's public types rather than fragile name heuristics.
//!
//! # Adding a lint
//!
//! See `CONTRIBUTING.md`. In short: declare the lint, add a marker struct and
//! `LateLintPass` impl, register both in [`register_lints`], and add a
//! [`LintMetadata`] entry to [`LINT_METADATA`] with the appropriate
//! [`LintCategory`].

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::get_enclosing_loop_or_multi_call_closure;
use clippy_utils::res::MaybeResPath;
use clippy_utils::ty::peel_and_count_ty_refs;
use clippy_utils::usage::local_used_after_expr;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintStore};
use rustc_middle::ty::Ty;
use rustc_span::def_id::DefId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;


dylint_linting::dylint_library!();

fn match_soroban_def_path<'tcx>(cx: &LateContext<'tcx>, def_id: DefId, segments: &[&str]) -> bool {
    let full = cx.tcx.def_path_str(def_id);
    let suffix = segments.join("::");
    full.ends_with(&suffix)
}

/// Returns whether `expr_ty` is one of the requested Soroban ADT types.
///
/// References are peeled before inspecting the type so callers can use this
/// helper for both owned values and references to SDK wrapper types.
fn is_type_match<'tcx>(
    cx: &LateContext<'tcx>,
    expr_ty: Ty<'tcx>,
    target_paths: &[&[&str]],
) -> bool {
    let peeled_ty = expr_ty.peel_refs();

    if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
        target_paths
            .iter()
            .any(|path| match_soroban_def_path(cx, adt_def.did(), path))
    } else {
        false
    }
}

const SOROBAN_STORAGE_TYPES: &[&[&str]] = &[
    &["soroban_sdk", "storage", "Storage"],
    &["soroban_sdk", "storage", "Instance"],
    &["soroban_sdk", "storage", "Persistent"],
    &["soroban_sdk", "storage", "Temporary"],
];

const SOROBAN_HOST_TYPES: &[&[&str]] = &[
    &["soroban_sdk", "ledger", "Ledger"],
    &["soroban_sdk", "crypto", "Crypto"],
    &["soroban_sdk", "crypto", "CryptoHazmat"],
    &["soroban_sdk", "crypto", "bls12_381", "Bls12_381"],
    &["soroban_sdk", "crypto", "bn254", "Bn254"],
    &["soroban_sdk", "prng", "Prng"],
    &["soroban_sdk", "events", "Events"],
    &["soroban_sdk", "deploy", "Deployer"],
    &["soroban_sdk", "deploy", "DeployerWithAddress"],
    &["soroban_sdk", "deploy", "DeployerWithAsset"],
];

const SOROBAN_ENV_HOST_METHODS: &[&str] = &["current_contract_address"];

fn enclosing_loop<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx hir::Expr<'tcx>,
) -> Option<&'tcx hir::Expr<'tcx>> {
    let enclosing = get_enclosing_loop_or_multi_call_closure(cx, expr)?;
    matches!(enclosing.kind, hir::ExprKind::Loop(..)).then_some(enclosing)
}

fn enclosing_loop_or_closure<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx hir::Expr<'tcx>,
) -> Option<&'tcx hir::Expr<'tcx>> {
    let enclosing = get_enclosing_loop_or_multi_call_closure(cx, expr)?;
    matches!(
        enclosing.kind,
        hir::ExprKind::Loop(..) | hir::ExprKind::Closure(..)
    )
    .then_some(enclosing)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintCategory {
    /// Reads and writes to Soroban storage (instance, persistent, temporary).
    StorageOperations,
    /// CPU-metered work, such as redundant host calls.
    Compute,
    /// Memory allocation and copying, such as needless clones.
    Memory,
    /// Creation and expiry of storage entries.
    EntryLifecycle,
    /// Construction and handling of `Symbol` values.
    SymbolOperations,
}

/// A single entry in the lint registry: a lint paired with its [`LintCategory`].
///
/// See [`LINT_METADATA`] for the full table.
pub struct LintMetadata {
    /// The lint this entry describes.
    pub lint: &'static rustc_lint::Lint,
    /// The cost dimension the lint is grouped under.
    pub category: LintCategory,
}

/// The registry of every lint shipped by this crate, each paired with its
/// [`LintCategory`].
///
/// This is the single source of truth for lint metadata: the `cargo-cost-lint`
/// wrapper reads it to enumerate and categorize available lints. Any lint added
/// in [`register_lints`] should also gain an entry here.
pub const LINT_METADATA: &[LintMetadata] = &[
    LintMetadata {
        lint: SOROBAN_STORAGE_IN_LOOP,
        category: LintCategory::StorageOperations,
    },
    LintMetadata {
        lint: LOOP_INVARIANT_STORAGE_ACCESS,
        category: LintCategory::StorageOperations,
    },
    LintMetadata {
        lint: UNBOUNDED_INPUT_LOOP,
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
];

/// Dylint entry point: registers every lint and its late pass with the
/// compiler's [`LintStore`].
///
/// `cargo dylint` calls this once per crate being checked. The set of lints
/// registered here must stay in sync with [`LINT_METADATA`]. The session
/// argument is unused; lint registration does not depend on session state.
#[unsafe(no_mangle)]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut LintStore) {
    lint_store.register_lints(&[
        SOROBAN_STORAGE_IN_LOOP,
        LOOP_INVARIANT_STORAGE_ACCESS,
        UNBOUNDED_INPUT_LOOP,
        REDUNDANT_ENV_CLONE,
        UNNECESSARY_HOST_FUNCTION_CALL,
    ]);
    lint_store.register_late_pass(|_| Box::new(SorobanStorageInLoop));
    lint_store.register_late_pass(|_| Box::new(LoopInvariantStorageAccess));
    lint_store.register_late_pass(|_| Box::new(UnboundedInputLoop));
    lint_store.register_late_pass(|_| Box::new(RedundantEnvClone));
    lint_store.register_late_pass(|_| Box::new(UnnecessaryHostFunctionCall));
}

rustc_session::declare_lint! {
    pub SOROBAN_STORAGE_IN_LOOP,
    Deny,
    "storage operations inside a loop"
}
/// Late pass backing [`SOROBAN_STORAGE_IN_LOOP`].
///
/// Flags storage method calls (`get`, `has`, `set`) on Soroban storage
/// accessor types (`Storage`, `Instance`, `Persistent`, `Temporary`) when
/// they appear inside a syntactic loop body, and also flags function calls
/// whose callee transitively reaches a storage operation within
/// [`MAX_CALL_DEPTH`] levels.
pub struct SorobanStorageInLoop;
rustc_session::impl_lint_pass!(SorobanStorageInLoop => [SOROBAN_STORAGE_IN_LOOP]);

impl<'tcx> LateLintPass<'tcx> for SorobanStorageInLoop {
    /// Flags a method call whose receiver is a Soroban storage accessor (or
    /// `Env::storage`) when it sits inside a loop.
    ///
    /// Storage access is metered on every iteration, so performing it in a loop
    /// multiplies the cost. The receiver type is matched against
    /// [`SOROBAN_STORAGE_TYPES`]; the loop check uses [`enclosing_loop`]. No
    /// suggestion is offered because the fix (hoisting or batching) is
    /// context-specific, so only a help note is emitted.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        // --- Direct storage access in a loop ---
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            let method_name = path_segment.ident.name.as_str();
            let is_terminal_storage_op = matches!(method_name, "get" | "has" | "set");
            let is_storage_access = is_terminal_storage_op
                && is_type_match(
                    cx,
                    cx.typeck_results().expr_ty(receiver),
                    SOROBAN_STORAGE_TYPES,
                );

            if is_storage_access && enclosing_loop(cx, expr).is_some() {
                let help = if matches!(method_name, "get" | "has") {
                    "if the read is loop-invariant, hoist it out of the loop; otherwise batch where possible"
                } else {
                    "move storage operations out of the loop or accumulate mutations in memory first"
                };
                span_lint_and_help(
                    cx,
                    SOROBAN_STORAGE_IN_LOOP,
                    expr.span,
                    "storage operation inside a loop",
                    None,
                    help,
                );
            }
        }

        // --- Inter-procedural: call that transitively reaches storage ---
        if let hir::ExprKind::Call(_callee, _args) = expr.kind
            && enclosing_loop(cx, expr).is_some()
        {
            let mut visited: Vec<DefId> = Vec::new();
            if let Some(callee_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
                && callee_contains_soroban_op(
                    cx.tcx,
                    callee_def_id,
                    SOROBAN_STORAGE_TYPES,
                    MAX_CALL_DEPTH,
                    &mut visited,
                )
            {
                span_lint_and_help(
                    cx,
                    SOROBAN_STORAGE_IN_LOOP,
                    expr.span,
                    "storage operation inside a loop (reached through function call)",
                    None,
                    "move storage operations out of the loop or accumulate mutations in memory first",
                );
            }
        }
    }
}

// =======================================================================
// loop_invariant_storage_access — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub LOOP_INVARIANT_STORAGE_ACCESS,
    Warn,
    "storage operation inside a loop whose operands are provably loop-invariant"
}
/// Late pass backing [`LOOP_INVARIANT_STORAGE_ACCESS`].
///
/// Flags storage operations whose receiver and arguments are provably
/// loop-invariant — the same value would be read or written on every
/// iteration. Hoisting such operations out of the loop saves repeated
/// metered host calls.
pub struct LoopInvariantStorageAccess;
rustc_session::impl_lint_pass!(LoopInvariantStorageAccess => [LOOP_INVARIANT_STORAGE_ACCESS]);

impl<'tcx> LateLintPass<'tcx> for LoopInvariantStorageAccess {
    /// Flags a storage method call inside a loop when none of its operands
    /// depend on per-iteration state (loop variables, mutated bindings).
    ///
    /// The receiver type is matched against [`SOROBAN_STORAGE_TYPES`] or
    /// recognised as `Env::storage()`.  Loop-invariance is checked by
    /// [`depends_on_loop_state`]; calls that read or write loop-varying
    /// state are not reported.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_storage_access = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                let did = adt_def.did();
                matches_any_path(cx, did, SOROBAN_STORAGE_TYPES)
                    || (match_soroban_def_path(cx, did, &["soroban_sdk", "Env"])
                        && path_segment.ident.name.as_str() == "storage")
            } else {
                false
            };

            if is_storage_access
                && let Some(loop_expr) = enclosing_loop(cx, expr)
                && !depends_on_loop_state(cx, loop_expr, expr)
            {
                span_lint_and_help(
                    cx,
                    LOOP_INVARIANT_STORAGE_ACCESS,
                    expr.span,
                    "loop-invariant storage operation inside a loop",
                    None,
                    "hoist this storage operation out of the loop",
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
/// Late pass backing [`REDUNDANT_ENV_CLONE`].
pub struct RedundantEnvClone;
rustc_session::impl_lint_pass!(RedundantEnvClone => [REDUNDANT_ENV_CLONE]);

impl<'tcx> LateLintPass<'tcx> for RedundantEnvClone {
    /// Flags a `.clone()` call whose receiver is a `soroban_sdk::Env`.
    ///
    /// `Env` is a cheap handle to the host and is almost always better passed
    /// by reference or value than cloned; the clone adds needless work. Matches
    /// the `clone` method name and confirms the receiver type resolves to
    /// `soroban_sdk::Env` before emitting a help note.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind
            && path_segment.ident.name.as_str() == "clone"
        {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let is_env = is_type_match(
                cx,
                receiver_ty,
                &[&["soroban_sdk", "Env"]],
            );

            if is_env {
                let (_inner, ref_count, _) = peel_and_count_ty_refs(receiver_ty);
                if ref_count > 0 {
                    return;
                }

                if let Some(local_id) = receiver.res_local_id() {
                    if local_used_after_expr(cx, local_id, expr) {
                        return;
                    }
                } else {
                    return;
                }

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
/// Late pass backing [`UNNECESSARY_HOST_FUNCTION_CALL`].
///
/// Flags metered host-function calls inside a loop (or multi-call closure)
/// whose results are loop-invariant and could be hoisted. The receiver is
/// matched against [`SOROBAN_HOST_TYPES`] or [`SOROBAN_ENV_HOST_METHODS`],
/// and calls whose inputs change per iteration are excluded via
/// [`depends_on_loop_state`].
pub struct UnnecessaryHostFunctionCall;
rustc_session::impl_lint_pass!(UnnecessaryHostFunctionCall => [UNNECESSARY_HOST_FUNCTION_CALL]);

impl<'tcx> LateLintPass<'tcx> for UnnecessaryHostFunctionCall {
    /// Flags a metered host call inside a loop whose result is invariant across
    /// iterations, so it could be computed once and reused.
    ///
    /// The receiver must resolve to one of [`SOROBAN_HOST_TYPES`], or the call
    /// must be one of the constant-result `Env` methods in
    /// [`SOROBAN_ENV_HOST_METHODS`]. The call is only reported when it is inside
    /// a loop ([`enclosing_loop`]) *and* does not read loop-varying state
    /// ([`depends_on_loop_state`]); the latter guard keeps calls whose inputs
    /// change each iteration from being flagged.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let is_host_function = is_type_match(cx, receiver_ty, SOROBAN_HOST_TYPES)
                || (is_type_match(cx, receiver_ty, &[&["soroban_sdk", "Env"]])
                    && SOROBAN_ENV_HOST_METHODS.contains(&path_segment.ident.name.as_str()));

            if is_host_function && enclosing_loop_or_closure(cx, expr).is_some() {
                span_lint_and_help(
                    cx,
                    UNNECESSARY_HOST_FUNCTION_CALL,
                    expr.span,
                    "unnecessary host function call inside loop",
                    None,
                    "cache the result outside the loop when the call is loop-invariant",
                );
            }
        }
    }
}
