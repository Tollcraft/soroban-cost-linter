#![feature(rustc_private)]
#![warn(unused_extern_crates)]
// Intra-doc links below reference private items (helpers, the const
// arrays, and the trait impls). Those references are intentional —
// rustdoc would otherwise emit `private_intra_doc_links` warnings for
// every bracketed [`foo`] in this file.
#![allow(rustdoc::private_intra_doc_links)]

//! Soroban-specific lints that detect host-call cost anti-patterns in Rust
//! smart contracts that target the Stellar Soroban runtime.
//!
//! Each lint implements `rustc_lint::LateLintPass` by walking the HIR and
//! matching structural patterns in `soroban_sdk` calls. Detection is
//! intentionally input-independent so that false positives collapse to
//! "almost certainly a bug"; patterns that depend on per-iteration state
//! are passed over (see `depends_on_loop_state`).
//!
//! The `cargo-cost-lint` CLI reads
//! [`LINT_METADATA`] to enumerate the available lints in `budget.toml` and
//! on the `--list` command line, so adding a new lint requires three
//! coordinated edits: a [`declare_lint!`] entry, a row in
//! [`LINT_METADATA`], and a registration call in [`register_lints`].

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
use clippy_utils::usage::mutated_variables;
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{HirId, HirIdSet};
use rustc_lint::{LateContext, LateLintPass, LintStore};
use rustc_span::def_id::DefId;

dylint_linting::dylint_library!();

fn match_soroban_def_path<'tcx>(cx: &LateContext<'tcx>, def_id: DefId, segments: &[&str]) -> bool {
    let full = cx.tcx.def_path_str(def_id);
    let suffix: String = segments.join("::");
    full.ends_with(&suffix)
}

/// Soroban storage accessor types. Every method call on one of these reaches
/// the host's storage subsystem.
const SOROBAN_STORAGE_TYPES: &[&[&str]] = &[
    &["soroban_sdk", "storage", "Storage"],
    &["soroban_sdk", "storage", "Instance"],
    &["soroban_sdk", "storage", "Persistent"],
    &["soroban_sdk", "storage", "Temporary"],
];

/// Soroban host accessor types reachable from `Env`. A method call on any of
/// them crosses the guest/host boundary and is metered, so repeating it inside
/// a loop with unchanged inputs is wasted CPU budget.
///
/// `soroban_sdk::storage::*` is deliberately absent: storage operations in a
/// loop are reported by [`SOROBAN_STORAGE_IN_LOOP`] instead.
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

/// Host calls that live directly on `Env` rather than on an accessor type, and
/// whose result is constant for the whole invocation.
///
/// The accessor methods themselves (`Env::ledger`, `Env::crypto`, ...) are not
/// listed: they only build a wrapper value, the metered work happens in the
/// method called on the wrapper. Argument-taking `Env` methods such as
/// `invoke_contract` or `authorize_as_current_contract` are also excluded
/// because their cost is inherent to what the loop is doing.
const SOROBAN_ENV_HOST_METHODS: &[&str] = &["current_contract_address"];

/// Soroban SDK container types. Growth-method calls (append, push_back, insert,
/// extend_from_array) on these inside a loop reallocate host-side state on
/// every iteration.
const SOROBAN_CONTAINER_TYPES: &[&[&str]] = &[
    &["soroban_sdk", "Bytes"],
    &["soroban_sdk", "Vec"],
    &["soroban_sdk", "Map"],
];

/// Methods on [`SOROBAN_CONTAINER_TYPES`] that grow the container's backing
/// buffer, causing increasingly expensive host-side work per call.
const BYTES_APPEND_METHODS: &[&str] = &["append", "push_back", "insert", "extend_from_array"];

fn matches_any_path<'tcx>(cx: &LateContext<'tcx>, def_id: DefId, paths: &[&[&str]]) -> bool {
    paths
        .iter()
        .any(|segments| match_soroban_def_path(cx, def_id, segments))
}

/// Collects the `HirId`s of every binding introduced inside the visited
/// subtree, e.g. the loop variable of a `for` loop or a per-iteration `let`.
#[derive(Default)]
struct BindingCollector {
    bindings: HirIdSet,
}

impl<'tcx> Visitor<'tcx> for BindingCollector {
    fn visit_pat(&mut self, pat: &'tcx hir::Pat<'tcx>) {
        if let hir::PatKind::Binding(_, hir_id, _, _) = pat.kind {
            self.bindings.insert(hir_id);
        }
        intravisit::walk_pat(self, pat);
    }
}

/// Collects the `HirId`s of every local read in the visited subtree.
#[derive(Default)]
struct LocalReadCollector {
    reads: Vec<HirId>,
}

impl<'tcx> Visitor<'tcx> for LocalReadCollector {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::Path(hir::QPath::Resolved(None, path)) = expr.kind
            && let hir::def::Res::Local(hir_id) = path.res
        {
            self.reads.push(hir_id);
        }
        intravisit::walk_expr(self, expr);
    }
}

/// Whether `call` — receiver chain and arguments included — reads anything that
/// changes from iteration to iteration of `loop_expr`.
///
/// Such a call is doing real per-iteration work, so hoisting it out of the loop
/// would change behaviour and it must not be reported. The answer errs towards
/// "depends": when the mutation analysis cannot give a verdict, the call is
/// treated as loop-dependent and stays unreported.
///
/// Known gaps, all of which cause a call to be reported rather than skipped:
/// bindings and mutations inside a closure body nested in the loop are not
/// seen, and mutation through a raw pointer or interior mutability (`RefCell`,
/// `Cell`) is not tracked.
fn depends_on_loop_state<'tcx>(
    cx: &LateContext<'tcx>,
    loop_expr: &'tcx hir::Expr<'tcx>,
    call: &'tcx hir::Expr<'tcx>,
) -> bool {
    let Some(mutated) = mutated_variables(loop_expr, cx) else {
        return true;
    };

    let mut bound = BindingCollector::default();
    bound.visit_expr(loop_expr);

    let mut read = LocalReadCollector::default();
    read.visit_expr(call);

    read.reads
        .iter()
        .any(|hir_id| bound.bindings.contains(hir_id) || mutated.contains(hir_id))
}

/// Whether `expr` sits directly inside a loop body, returning that loop.
///
/// A call inside a closure that the loop calls is not reported: the closure may
/// well be defined elsewhere, and the receiver is out of reach for the
/// loop-dependence analysis.
fn enclosing_loop<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx hir::Expr<'tcx>,
) -> Option<&'tcx hir::Expr<'tcx>> {
    let enclosing = get_enclosing_loop_or_multi_call_closure(cx, expr)?;
    matches!(enclosing.kind, hir::ExprKind::Loop(..)).then_some(enclosing)
}

/// High-level cost category a lint belongs to. Surfaced by `cargo-cost-lint`
/// to group warnings in the `--report` output and to label `budget.toml`
/// rows under their category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintCategory {
    /// Direct ledger reads/writes via the `Storage`, `Instance`, `Persistent`,
    /// or `Temporary` accessors.
    StorageOperations,
    /// Host functions that cross the Wasm guest/host boundary and burn CPU
    /// budget with each call (`ledger`, `crypto`, `events`, ...).
    Compute,
    /// Guest- or host-side allocations that grow with input size, including
    /// repeated `soroban_sdk::Bytes` / `Vec` / `Map` mutations.
    Memory,
    /// Lifecycle of contract entries: authorisation, deployment, removal.
    EntryLifecycle,
    /// Construction and reuse of `soroban_sdk::Symbol` values.
    SymbolOperations,
}

/// Row in the lint registry. Pairs the [`rustc_lint::Lint`] static declared
/// by this crate with the [`LintCategory`] the CLI uses to route the
/// diagnostic and the `budget.toml` row.
pub struct LintMetadata {
    /// The lint description registered with rustc; surfaced verbatim in
    /// `cargo build` output and in `cargo-cost-lint`'s `--list`.
    pub lint: &'static rustc_lint::Lint,
    /// Which [`LintCategory`] the lint belongs to.
    pub category: LintCategory,
}

/// Registry of every lint exposed by this crate, in declaration order.
///
/// `cargo-cost-lint` iterates this slice to render the `--list` output and
/// to map `[level.<name>]` rows in `budget.toml` back to rustc-level lint
/// names. New lints must be added here and in `register_lints`, otherwise
/// the CLI will be unable to configure them.
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
        lint: BYTES_APPEND_IN_LOOP,
        category: LintCategory::Memory,
    },
];

/// `dylint` entry point. Registers every lint declared by this crate with
/// the supplied [`LintStore`] and installs the concrete
/// [`LateLintPass`] implementations that drive detection.
///
/// The `#[unsafe(no_mangle)]` attribute is required so dylint can find
/// this symbol regardless of its Rust name mangling; do not rename the
/// function without also updating dylint's lookup table.
#[unsafe(no_mangle)]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut LintStore) {
    lint_store.register_lints(&[
        SOROBAN_STORAGE_IN_LOOP,
        REDUNDANT_ENV_CLONE,
        UNNECESSARY_HOST_FUNCTION_CALL,
        HOST_IN_LOOP,
        SYMBOL_NEW_FOR_SHORT_LITERAL,
        BYTES_APPEND_IN_LOOP,
    ]);
    lint_store.register_late_pass(|_| Box::new(SorobanStorageInLoop));
    lint_store.register_late_pass(|_| Box::new(RedundantEnvClone));
    lint_store.register_late_pass(|_| Box::new(UnnecessaryHostFunctionCall));
    lint_store.register_late_pass(|_| Box::new(HostInLoop));
    lint_store.register_late_pass(|_| Box::new(SymbolNewForShortLiteral));
    lint_store.register_late_pass(|_| Box::new(BytesAppendInLoop));
}

/// Flags any Soroban storage accessor method call (including
/// `Env::storage()`, which returns a `Storage` wrapper) that sits
/// directly inside a loop body. Each iteration pays a separate storage
/// cost, and the visible structural pattern almost always indicates an
/// unintended per-iteration expense.
rustc_session::declare_lint! {
    pub SOROBAN_STORAGE_IN_LOOP,
    Warn,
    "storage operations inside a loop"
}
/// Concrete pass that fires [`SOROBAN_STORAGE_IN_LOOP`].
pub struct SorobanStorageInLoop;
rustc_session::impl_lint_pass!(SorobanStorageInLoop => [SOROBAN_STORAGE_IN_LOOP]);

/// Detection: for every `expr.kind == MethodCall`, peel references off the
/// receiver's type and look for one of [`SOROBAN_STORAGE_TYPES`], or for
/// `Env::storage()`, which is the documented entry point for custom
/// storage. A match is reported only when [`enclosing_loop`] returns
/// `Some`.
impl<'tcx> LateLintPass<'tcx> for SorobanStorageInLoop {
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

            if is_storage_access && enclosing_loop(cx, expr).is_some() {
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

/// Flags `.clone()` calls on a `soroban_sdk::Env` value. `Env` is a
/// guest-side handle — cloning it produces no new host resource and
/// merely wastes a few instructions, so the call is almost always either
/// a typo or code cargo-culted from a non-Soroban codebase.
rustc_session::declare_lint! {
    pub REDUNDANT_ENV_CLONE,
    Warn,
    "redundant clone on Env object"
}
/// Concrete pass that fires [`REDUNDANT_ENV_CLONE`].
pub struct RedundantEnvClone;
rustc_session::impl_lint_pass!(RedundantEnvClone => [REDUNDANT_ENV_CLONE]);

/// Detection: for every `MethodCall` whose segment is named `clone`, peel
/// references off the receiver and check whether the underlying ADT
/// resolves to `soroban_sdk::Env`. No loop analysis is needed — the lint
/// is purely structural.
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

/// Flags host accessor calls inside a loop whose result does not depend on
/// per-iteration state and could be hoisted out. Each iteration pays the
/// full cross-boundary cost; in aggregate this becomes the dominant
/// expense of any contract that touches `ledger`, `crypto`, `events`, or
/// `prng` inside a loop by mistake.
rustc_session::declare_lint! {
    pub UNNECESSARY_HOST_FUNCTION_CALL,
    Warn,
    "unnecessary host function call inside loop"
}
/// Concrete pass that fires [`UNNECESSARY_HOST_FUNCTION_CALL`].
pub struct UnnecessaryHostFunctionCall;
rustc_session::impl_lint_pass!(UnnecessaryHostFunctionCall => [UNNECESSARY_HOST_FUNCTION_CALL]);

/// Flags any construction of a `Host` value inside a loop. The `Host`
/// handle is normally stashed in a contract-static — recreating it per
/// iteration is almost always a leftover from refactoring.
rustc_session::declare_lint! {
    pub HOST_IN_LOOP,
    Warn,
    "use of Host object inside a loop"
}
/// Concrete pass that fires [`HOST_IN_LOOP`].
pub struct HostInLoop;
rustc_session::impl_lint_pass!(HostInLoop => [HOST_IN_LOOP]);

/// Detection: for every `MethodCall`, peel the receiver's reference
/// layers. The call is reported iff:
///
/// 1. The receiver type resolves to one of [`SOROBAN_HOST_TYPES`] (the
///    sibling accessor types) or to `soroban_sdk::Env` whose matched
///    segment is in [`SOROBAN_ENV_HOST_METHODS`] (rare value-returning
///    methods on `Env` itself).
/// 2. [`enclosing_loop`] returns `Some`.
/// 3. [`depends_on_loop_state`] returns `false`, i.e. the call's inputs
///    are loop-invariant and the result could safely be cached outside
///    the loop.
impl<'tcx> LateLintPass<'tcx> for UnnecessaryHostFunctionCall {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_host_function = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                let did = adt_def.did();
                matches_any_path(cx, did, SOROBAN_HOST_TYPES)
                    || (match_soroban_def_path(cx, did, &["soroban_sdk", "Env"])
                        && SOROBAN_ENV_HOST_METHODS.contains(&path_segment.ident.name.as_str()))
            } else {
                false
            };

            if is_host_function
                && let Some(loop_expr) = enclosing_loop(cx, expr)
                && !depends_on_loop_state(cx, loop_expr, expr)
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

/// Detection: for every `MethodCall`, peel references off the receiver
/// and check whether the underlying ADT resolves to `host::Host`. A match
/// is reported only when [`enclosing_loop`] returns `Some`. The check is
/// intentionally narrower than [`UNNECESSARY_HOST_FUNCTION_CALL`] so the
/// two diagnostics do not overlap when both triggers are present.
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

            if is_host && enclosing_loop(cx, expr).is_some() {
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

/// Flags `Symbol::new(&env, "literal")` calls whose literal satisfies the
/// length and character constraints accepted by the `symbol_short!` macro.
/// The macro lifts construction to compile time, eliminating both the
/// per-call host invocation and the runtime string-validation cost.
rustc_session::declare_lint! {
    pub SYMBOL_NEW_FOR_SHORT_LITERAL,
    Warn,
    "Symbol::new used with a short literal that could use symbol_short! macro"
}
/// Concrete pass that fires [`SYMBOL_NEW_FOR_SHORT_LITERAL`].
pub struct SymbolNewForShortLiteral;
rustc_session::impl_lint_pass!(SymbolNewForShortLiteral => [SYMBOL_NEW_FOR_SHORT_LITERAL]);

/// Detection: find every `Call` whose callee resolves to
/// `soroban_sdk::Symbol::new` and whose second argument is a string
/// literal. The literal is accepted iff [`is_valid_short_symbol`] returns
/// `true`. When the source snippet for the literal is available, a
/// machine-applicable `symbol_short!(literal)` suggestion is emitted;
/// otherwise only the help message is shown.
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

// =======================================================================
// bytes_append_in_loop — Lint
// =======================================================================

/// Flags repeated `.append`, `.push_back`, `.insert`, or
/// `.extend_from_array` calls on a Soroban container (`Bytes`, `Vec`,
/// `Map`) inside a loop. Each call reallocates host-side state, so the
/// per-iteration cost rises with the iteration count and quickly becomes
/// the dominant expense of the contract.
rustc_session::declare_lint! {
    pub BYTES_APPEND_IN_LOOP,
    Warn,
    "repeatedly growing SDK containers inside loops"
}
/// Concrete pass that fires [`BYTES_APPEND_IN_LOOP`].
pub struct BytesAppendInLoop;
rustc_session::impl_lint_pass!(BytesAppendInLoop => [BYTES_APPEND_IN_LOOP]);

/// Detection: for every `MethodCall` whose segment is one of
/// [`BYTES_APPEND_METHODS`], peel references off the receiver and confirm
/// the ADT belongs to [`SOROBAN_CONTAINER_TYPES`]. A match is reported
/// only when [`enclosing_loop`] returns `Some`. We deliberately do **not**
/// attempt to detect whether the loop could be batched — that reasoning
/// is runtime-dependent and would inflate the false-positive rate.
impl<'tcx> LateLintPass<'tcx> for BytesAppendInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            let method_name = path_segment.ident.name.as_str();
            if !BYTES_APPEND_METHODS.contains(&method_name) {
                return;
            }

            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_container = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                matches_any_path(cx, adt_def.did(), SOROBAN_CONTAINER_TYPES)
            } else {
                false
            };

            if is_container && enclosing_loop(cx, expr).is_some() {
                span_lint_and_help(
                    cx,
                    BYTES_APPEND_IN_LOOP,
                    expr.span,
                    "repeatedly growing SDK container inside a loop",
                    None,
                    "accumulate values in native Rust collections first, then batch host \
                     operations or convert to SDK containers once after the loop; \
                     pre-size where practical",
                );
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

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
