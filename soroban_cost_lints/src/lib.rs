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

use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg};
use clippy_utils::get_enclosing_loop_or_multi_call_closure;
use clippy_utils::res::MaybeResPath;
use clippy_utils::source::snippet_opt;
use clippy_utils::ty::peel_and_count_ty_refs;
use clippy_utils::usage::local_used_after_expr;
use clippy_utils::usage::mutated_variables;
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_hir::intravisit::{self, FnKind, Visitor};
use rustc_hir::{FnDecl, HirId, HirIdSet};
use rustc_lint::{LateContext, LateLintPass, LintStore};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;

dylint_linting::dylint_library!();

// ---------------------------------------------------------------------------
// Per-DefId cache of `def_path_str` so that the expensive full-path
// formatting happens at most once per unique DefId instead of for every
// method-call expression the lints visit.
// ---------------------------------------------------------------------------

thread_local! {
    static DEF_PATH_CACHE: RefCell<HashMap<DefId, String>> = RefCell::new(HashMap::new());
}

fn cached_def_path_str(tcx: TyCtxt<'_>, def_id: DefId) -> String {
    DEF_PATH_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache
            .entry(def_id)
            .or_insert_with(|| tcx.def_path_str(def_id))
            .clone()
    })
}

/// Compares `def_id` against the canonical definition path `segments`.
/// The hot `def_path_str` call is cached per `DefId` so repeated checks on
/// the same type (e.g. `Env`, `Bytes`) avoid re-formatting the full path.
fn match_soroban_def_path(cx: &LateContext<'_>, def_id: DefId, segments: &[&str]) -> bool {
    let full = cached_def_path_str(cx.tcx, def_id);
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

/// Soroban collection types that support linear-time scanning operations.
const SOROBAN_COLLECTION_TYPES: &[&[&str]] = &[
    &["soroban_sdk", "vec", "Vec"],
    &["soroban_sdk", "map", "Map"],
];

const LINEAR_SCAN_METHODS: &[&str] = &["contains", "position", "find"];

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

fn match_soroban_def_path_tcx(tcx: TyCtxt<'_>, def_id: DefId, segments: &[&str]) -> bool {
    let full = tcx.def_path_str(def_id);
    let suffix: String = segments.join("::");
    full.ends_with(&suffix)
}

fn matches_any_path_tcx(tcx: TyCtxt<'_>, def_id: DefId, paths: &[&[&str]]) -> bool {
    paths
        .iter()
        .any(|segments| match_soroban_def_path_tcx(tcx, def_id, segments))
}

/// Maximum call depth for inter-procedural analysis. Functions reachable
/// beyond this depth are not inspected; the analysis conservatively treats
/// them as not containing the target operation.
const MAX_CALL_DEPTH: u32 = 3;

/// Whether `def_id` (or a callee up to `depth_remaining` deep) performs a
/// Soroban storage/host operation matching `target_paths`.
///
/// # Conservative posture
///
/// External crates (`!def_id.is_local()`), opaque calls (trait methods,
/// function pointers, closures), cycle revisits, and exhausted depth all
/// return `false`. This errs on the side of **not** flagging, so a
/// cross-function lint never produces a false positive from incomplete
/// information.
fn callee_contains_soroban_op<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    target_paths: &[&[&str]],
    depth_remaining: u32,
    visited: &mut Vec<DefId>,
) -> bool {
    // External crate — we can't walk soroban-sdk or std
    if !def_id.is_local() {
        return false;
    }
    // Already visited in this call chain (cycle / recursion)
    if visited.contains(&def_id) {
        return false;
    }
    // Depth bound exhausted
    if depth_remaining == 0 {
        return false;
    }

    visited.push(def_id);

    let local_def_id = def_id.expect_local();
    let body_id = tcx
        .hir_node_by_def_id(local_def_id)
        .body_id()
        .expect("callee has no body");
    let body = tcx.hir_body(body_id);
    let typeck = tcx.typeck(local_def_id);

    let found = {
        let mut detector = CalleeStorageDetector {
            tcx,
            typeck,
            target_paths,
            depth_remaining: depth_remaining - 1,
            visited,
            found: false,
        };
        detector.visit_expr(body.value);
        detector.found
    };

    visited.pop();
    found
}

/// Visitor that walks a callee body looking for direct storage/host method
/// calls or nested calls that transitively reach one.
struct CalleeStorageDetector<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    typeck: &'tcx rustc_middle::ty::TypeckResults<'tcx>,
    target_paths: &'a [&'a [&'a str]],
    depth_remaining: u32,
    visited: &'a mut Vec<DefId>,
    found: bool,
}

impl<'a, 'tcx> Visitor<'tcx> for CalleeStorageDetector<'a, 'tcx> {
    /// Visits an expression, flagging method calls on storage/host types and
    /// recursive calls that transitively reach such operations.
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        if self.found {
            return;
        }

        match expr.kind {
            hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) => {
                let receiver_ty = self.typeck.expr_ty(receiver);
                let peeled_ty = receiver_ty.peel_refs();

                if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                    let did = adt_def.did();
                    if matches_any_path_tcx(self.tcx, did, self.target_paths)
                        || (match_soroban_def_path_tcx(self.tcx, did, &["soroban_sdk", "Env"])
                            && path_segment.ident.name.as_str() == "storage")
                    {
                        self.found = true;
                        return;
                    }
                }
            }
            hir::ExprKind::Call(_callee, _args) => {
                // Check whether this nested call transitively reaches a
                // storage/host op.
                if let Some(callee_def_id) = self.typeck.type_dependent_def_id(expr.hir_id)
                    && callee_contains_soroban_op(
                        self.tcx,
                        callee_def_id,
                        self.target_paths,
                        self.depth_remaining,
                        self.visited,
                    )
                {
                    self.found = true;
                    return;
                }
            }
            _ => {}
        }

        intravisit::walk_expr(self, expr);
    }
}

/// Collects the `HirId`s of every binding introduced inside the visited
/// subtree, e.g. the loop variable of a `for` loop or a per-iteration `let`.
#[derive(Default)]
struct BindingCollector {
    bindings: HirIdSet,
}

impl<'tcx> Visitor<'tcx> for BindingCollector {
    /// Records the `HirId` of any binding pattern encountered, then recurses
    /// into sub-patterns so nested bindings (e.g. `(a, b)`) are all captured.
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
    /// Records the `HirId` of every resolved read of a local variable, i.e. a
    /// path expression that resolves to a `Res::Local`, then recurses into the
    /// rest of the expression tree.
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
        lint: SOROBAN_INEFFICIENT_BYTES_CONCAT,
        category: LintCategory::Compute,
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
        lint: CONTRACT_CALL_IN_LOOP,
        category: LintCategory::Compute,
    },
    LintMetadata {
        lint: SYMBOL_NEW_FOR_SHORT_LITERAL,
        category: LintCategory::SymbolOperations,
    },
    LintMetadata {
        lint: PERSISTENT_READ_WITHOUT_TTL_EXTENSION,
        category: LintCategory::EntryLifecycle,
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
        SOROBAN_INEFFICIENT_BYTES_CONCAT,
        REDUNDANT_ENV_CLONE,
        UNNECESSARY_HOST_FUNCTION_CALL,
        HOST_IN_LOOP,
        CONTRACT_CALL_IN_LOOP,
        SYMBOL_NEW_FOR_SHORT_LITERAL,
        PERSISTENT_READ_WITHOUT_TTL_EXTENSION,
    ]);
    lint_store.register_late_pass(|_| Box::new(SorobanStorageInLoop));
    lint_store.register_late_pass(|_| Box::new(SorobanInefficientBytesConcat));
    lint_store.register_late_pass(|_| Box::new(RedundantEnvClone));
    lint_store.register_late_pass(|_| Box::new(UnnecessaryHostFunctionCall));
    lint_store.register_late_pass(|_| Box::new(HostInLoop));
    lint_store.register_late_pass(|_| Box::new(ContractCallInLoop));
    lint_store.register_late_pass(|_| Box::new(SymbolNewForShortLiteral));
    lint_store.register_late_pass(|_| Box::new(PersistentReadWithoutTtlExtension));
}

/// Flags any Soroban storage accessor method call (including
/// `Env::storage()`, which returns a `Storage` wrapper) that sits
/// directly inside a loop body. Each iteration pays a separate storage
/// cost, and the visible structural pattern almost always indicates an
/// unintended per-iteration expense.
rustc_session::declare_lint! {
    pub SOROBAN_STORAGE_IN_LOOP,
    Deny,
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
    /// Flags a method call whose receiver is a Soroban storage accessor (or
    /// `Env::storage`) when it sits inside a loop.
    ///
    /// Storage access is metered on every iteration, so performing it in a loop
    /// multiplies the cost. The receiver type is matched against
    /// [`SOROBAN_STORAGE_TYPES`]; the loop-or-closure check uses
    /// [`enclosing_loop_or_closure`]. No suggestion is offered because the fix
    /// (hoisting or batching) is context-specific, so only a help note is emitted.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        // --- Direct storage access in a loop ---
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            // Only fire on terminal storage operations (`get` / `has` /
            // `set`). The intermediate calls — `env.storage()`,
            // `.instance()` / `.persistent()` / `.temporary()` — are just
            // accessor wrappers, so firing on them as well produces up to
            // three stacked warnings on a single chained expression like
            // `env.storage().instance().set(&k, &v)`. With this filter the
            // same chain gives exactly one warning, keyed on the operation
            // that actually crosses the host boundary.
            let method_name = path_segment.ident.name.as_str();
            let is_terminal_storage_op = matches!(method_name, "get" | "has" | "set");

            let is_storage_access = is_terminal_storage_op
                && if let rustc_middle::ty::Adt(adt_def, _) =
                    cx.typeck_results().expr_ty(receiver).peel_refs().kind()
                {
                    matches_any_path(cx, adt_def.did(), SOROBAN_STORAGE_TYPES)
                } else {
                    false
                };

            if is_storage_access && enclosing_loop_or_closure(cx, expr).is_some() {
                // Reads (`get`, `has`) and writes (`set`) deserve different
                // advice: writes can be buffered and flushed once after the
                // loop, but a loop-variant read cannot be accumulated — the
                // user typically needs to hoist a loop-invariant read or
                // batch where possible.
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
            && enclosing_loop_or_closure(cx, expr).is_some()
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

/// Flags `.clone()` calls on a `soroban_sdk::Env` value. `Env` is a
/// guest-side handle — cloning it produces no new host resource and
/// merely wastes a few instructions, so the call is almost always either
/// a typo or code cargo-culted from a non-Soroban codebase.
rustc_session::declare_lint! {
    pub SOROBAN_INEFFICIENT_BYTES_CONCAT,
    Warn,
    "inefficient Bytes concatenation inside a loop"
}
pub struct SorobanInefficientBytesConcat;
rustc_session::impl_lint_pass!(SorobanInefficientBytesConcat => [SOROBAN_INEFFICIENT_BYTES_CONCAT]);

impl<'tcx> LateLintPass<'tcx> for SorobanInefficientBytesConcat {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            let method_name = path_segment.ident.name.as_str();
            // Only flag concatenation-related methods: push_back and append
            if method_name != "push_back" && method_name != "append" {
                return;
            }

            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_bytes = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                let did = adt_def.did();
                match_soroban_def_path(cx, did, &["soroban_sdk", "Bytes"])
                    || match_soroban_def_path(cx, did, &["soroban_sdk", "bytes", "Bytes"])
            } else {
                false
            };

            if is_bytes
                && let Some(enclosing_expr) = get_enclosing_loop_or_multi_call_closure(cx, expr)
                && let hir::ExprKind::Loop(..) = enclosing_expr.kind
            {
                span_lint_and_help(
                    cx,
                    SOROBAN_INEFFICIENT_BYTES_CONCAT,
                    expr.span,
                    "inefficient Bytes concatenation inside a loop",
                    None,
                    "accumulate bytes in a Vec<u8> inside the loop and convert to Bytes once outside the loop via Bytes::from_slice",
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
/// Concrete pass that fires [`REDUNDANT_ENV_CLONE`].
pub struct RedundantEnvClone;
rustc_session::impl_lint_pass!(RedundantEnvClone => [REDUNDANT_ENV_CLONE]);

/// Detection: for every `MethodCall` whose segment is named `clone`, peel
/// references off the receiver and check whether the underlying ADT
/// resolves to `soroban_sdk::Env`. No loop analysis is needed — the lint
/// is purely structural.
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
            let is_env = if let Some(adt_def) = try_get_adt_def(cx, receiver) {
                match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "Env"])
            } else {
                false
            };

            if is_env {
                // Clone on &Env produces an owned Env from a reference — genuinely needed.
                let receiver_ty = cx.typeck_results().expr_ty(receiver);
                let (_inner, ref_count, _) = peel_and_count_ty_refs(receiver_ty);
                if ref_count > 0 {
                    return;
                }

                // If the receiver is a local binding that is still used after
                // the clone, the original and the clone are both live — skip.
                if let Some(local_id) = receiver.res_local_id() {
                    if local_used_after_expr(cx, local_id, expr) {
                        return;
                    }
                } else {
                    // Cannot statically determine whether the receiver is used
                    // after the clone — be conservative and skip.
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
            let is_host_function = if let Some(adt_def) = try_get_adt_def(cx, receiver) {
                let did = adt_def.did();
                matches_any_path(cx, did, SOROBAN_HOST_TYPES)
                    || (match_soroban_def_path(cx, did, &["soroban_sdk", "Env"])
                        && SOROBAN_ENV_HOST_METHODS.contains(&path_segment.ident.name.as_str()))
            } else {
                false
            };

            // Accept both syntactic loops (`for` / `while` / `loop`) and
            // multi-call closures (the body of `Iterator::for_each`, ...).
            // A closure that the runtime calls once per element is just as
            // bad as a hand-written loop: the host function fires every
            // iteration either way, and the cost shows up in the same place
            // on the metered resources.
            if is_host_function
                && let Some(loop_expr) = enclosing_loop_or_closure(cx, expr)
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
    /// Flags any method call whose receiver is a `host::Host` object inside a
    /// loop.
    ///
    /// Unlike [`UnnecessaryHostFunctionCall`], this pass does not attempt a
    /// loop-invariance analysis: any `Host` use in a loop is surfaced with a
    /// help note suggesting the call be moved out where possible.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(_path_segment, receiver, _args, _span) = expr.kind {
            let is_host = if let Some(adt_def) = try_get_adt_def(cx, receiver) {
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

rustc_session::declare_lint! {
    /// ### What it does
    /// Detects unnecessary `.to_bytes()` calls on the Soroban `String` object.
    pub UNNECESSARY_STRING_TO_BYTES,
    Warn,
    "unnecessary String to Bytes conversion"
}
pub struct UnnecessaryStringToBytes;
rustc_session::impl_lint_pass!(UnnecessaryStringToBytes => [UNNECESSARY_STRING_TO_BYTES]);

impl<'tcx> LateLintPass<'tcx> for UnnecessaryStringToBytes {
    /// Flags `.to_bytes()` calls on `soroban_sdk::String` values.
    ///
    /// Converting a `String` to `Bytes` is a metered host operation.  In
    /// many contexts the `String` can be used directly where `Bytes` is
    /// accepted, or a `Bytes` value can be constructed from the same data
    /// without the conversion overhead.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind
            && path_segment.ident.name.as_str() == "to_bytes"
        {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_string = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "String"])
            } else {
                false
            };

            if is_string {
                span_lint_and_help(
                    cx,
                    UNNECESSARY_STRING_TO_BYTES,
                    expr.span,
                    "unnecessary String to Bytes conversion",
                    None,
                    "use the String directly where Bytes is accepted, or construct Bytes directly instead",
                );
            }
        }
    }
}

// =======================================================================
// contract_call_in_loop — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub CONTRACT_CALL_IN_LOOP,
    Warn,
    "cross-contract invocation inside a loop"
}
pub struct ContractCallInLoop;
rustc_session::impl_lint_pass!(ContractCallInLoop => [CONTRACT_CALL_IN_LOOP]);

impl<'tcx> LateLintPass<'tcx> for ContractCallInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind
            && path_segment.ident.name.as_str() == "invoke_contract"
        {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_env = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "Env"])
            } else {
                false
            };

            if is_env && enclosing_loop(cx, expr).is_some() {
                span_lint_and_help(
                    cx,
                    CONTRACT_CALL_IN_LOOP,
                    expr.span,
                    "cross-contract call inside a loop",
                    None,
                    "add a bulk endpoint on the callee contract, or hoist this call out of the loop if its result is invariant across iterations",
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
    /// Flags `Symbol::new(&env, "literal")` when the literal is short enough to
    /// build at compile time with the `symbol_short!` macro.
    ///
    /// `Symbol::new` constructs the symbol at runtime, which is metered;
    /// `symbol_short!` produces it as a compile-time constant instead. The pass
    /// recognizes a two-argument call to `soroban_sdk::Symbol::new` whose second
    /// argument is a string literal accepted by [`is_valid_short_symbol`]. When
    /// the argument snippet is available it emits a machine-applicable
    /// suggestion; otherwise it falls back to a help note.
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
                let symbol_str = symbol.as_str();
                if is_valid_short_symbol(symbol_str) {
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
// unbounded_input_loop — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub UNBOUNDED_INPUT_LOOP,
    Warn,
    "loop bound derived from untrusted input with storage write in body"
}

/// Late pass backing [`UNBOUNDED_INPUT_LOOP`].
///
/// Flags loops whose iteration count is derived from a function parameter
/// (i.e. untrusted input) and whose body performs a storage write.  Such
/// loops can be abused to exhaust the contract's CPU/memory budget, so the
/// author should clamp the bound (e.g. with `.min(CONST)`) or validate the
/// input before using it as a loop bound.
#[derive(Default)]
pub struct UnboundedInputLoop;
rustc_session::impl_lint_pass!(UnboundedInputLoop => [UNBOUNDED_INPUT_LOOP]);

/// Collects the `HirId` of every function parameter pattern.
#[derive(Default)]
struct ParamHirIdCollector {
    params: HirIdSet,
}

impl<'tcx> Visitor<'tcx> for ParamHirIdCollector {
    /// Records the `HirId` of every binding pattern encountered, recursing
    /// into sub-patterns to capture destructured parameters.
    fn visit_pat(&mut self, pat: &'tcx hir::Pat<'tcx>) {
        if let hir::PatKind::Binding(_, hir_id, _, _) = pat.kind {
            self.params.insert(hir_id);
        }
        intravisit::walk_pat(self, pat);
    }
}

/// Whether `expr` is a clamping method call (`.min(CONST)` or
/// `.clamp(_, CONST)` where the constant is a literal).
#[allow(dead_code)]
fn is_clamped_by_constant(_cx: &LateContext<'_>, expr: &hir::Expr<'_>) -> bool {
    if let hir::ExprKind::MethodCall(path_segment, _receiver, args, _span) = expr.kind {
        let name = path_segment.ident.name.as_str();
        match name {
            "min" => args
                .first()
                .is_some_and(|a| matches!(a.kind, hir::ExprKind::Lit(_))),
            "clamp" => {
                args.len() >= 2 && matches!(args[args.len() - 1].kind, hir::ExprKind::Lit(_))
            }
            _ => false,
        }
    } else {
        false
    }
}

impl<'tcx> LateLintPass<'tcx> for UnboundedInputLoop {
    /// Visits each named function, collecting parameter `HirId`s and walking
    /// the body for loops whose bound references a function parameter and
    /// whose body contains a storage write.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        body: &'tcx hir::Body<'tcx>,
        _span: rustc_span::Span,
        _local_def_id: rustc_hir::def_id::LocalDefId,
    ) {
        // Only check named functions (not closures)
        if !matches!(kind, FnKind::ItemFn(..)) {
            return;
        }

        // Collect function parameter HirIds from body.params
        let mut collector = ParamHirIdCollector::default();
        for param in body.params {
            collector.visit_pat(param.pat);
        }
        if collector.params.is_empty() {
            return;
        }

        let mut walker = UnboundedLoopWalker {
            cx,
            param_ids: collector.params,
            within_loop: false,
            current_loop: None,
            flagged: HirIdSet::default(),
        };
        walker.visit_expr(body.value);
    }
}

/// Walker that traverses a function body to find loops with parameter-derived
/// bounds that contain storage writes.
struct UnboundedLoopWalker<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    param_ids: HirIdSet,
    within_loop: bool,
    current_loop: Option<(hir::HirId, bool)>,
    flagged: HirIdSet,
}

impl<'a, 'tcx> Visitor<'tcx> for UnboundedLoopWalker<'a, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        match expr.kind {
            // A Block that ends with a Loop: this is a for-loop or while-loop in
            // the desugared HIR. The preceding statements hold the iterator init /
            // condition from which we can extract the bound expression.
            hir::ExprKind::Block(block, _label)
                if let Some(tail) = block.expr
                    && matches!(tail.kind, hir::ExprKind::Loop(..)) =>
            {
                let bound_refers_to_param = self.block_stmts_contain_param_bound(block.stmts);
                let was_in_loop = self.within_loop;
                let prev_loop = self.current_loop;

                self.within_loop = true;
                self.current_loop = Some((expr.hir_id, bound_refers_to_param));

                // Visit iterator init statements (they're outside the loop body)
                for stmt in block.stmts {
                    intravisit::walk_stmt(self, stmt);
                }
                // Visit the loop body
                self.visit_expr(tail);

                self.within_loop = was_in_loop;
                self.current_loop = prev_loop;
            }

            // A free-standing Loop (not inside the for/while desugaring pattern)
            // e.g. `loop { ... }` — we can't determine the bound, skip.
            hir::ExprKind::Loop(body_block, _label, _loop_source, _span) => {
                let was_in_loop = self.within_loop;
                let prev_loop = self.current_loop;
                self.within_loop = true;
                // Bound is unknown for bare `loop { }`
                self.current_loop = Some((expr.hir_id, false));
                intravisit::walk_block(self, body_block);
                self.within_loop = was_in_loop;
                self.current_loop = prev_loop;
            }

            // Storage write detection — only relevant inside a loop
            hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) if self.within_loop => {
                let receiver_ty = self.cx.typeck_results().expr_ty(receiver);
                let peeled_ty = receiver_ty.peel_refs();

                let is_storage = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                    let did = adt_def.did();
                    matches_any_path(self.cx, did, SOROBAN_STORAGE_TYPES)
                        || (match_soroban_def_path(self.cx, did, &["soroban_sdk", "Env"])
                            && path_segment.ident.name.as_str() == "storage")
                } else {
                    false
                };

                if is_storage
                    && let Some((loop_id, bound_is_param)) = self.current_loop
                    && bound_is_param
                    && !self.flagged.contains(&loop_id)
                {
                    self.flagged.insert(loop_id);
                    span_lint_and_help(
                        self.cx,
                        UNBOUNDED_INPUT_LOOP,
                        expr.span,
                        "loop bound derives from an untrusted input with a storage write in the body",
                        None,
                        "clamp the loop bound with .min(CONST) or validate the input before using it as a loop bound",
                    );
                }

                intravisit::walk_expr(self, expr);
            }

            _ => {
                intravisit::walk_expr(self, expr);
            }
        }
    }
}

impl<'a, 'tcx> UnboundedLoopWalker<'a, 'tcx> {
    /// Walks the block's preceding statements (the iterator init in a
    /// desugared `for`/`while` loop) and returns `true` if any expression
    /// references a function parameter, indicating the loop bound is
    /// derived from untrusted input.
    fn block_stmts_contain_param_bound(&mut self, stmts: &'tcx [hir::Stmt<'tcx>]) -> bool {
        /// Scans expressions for reads of function parameters, stopping
        /// early once one is found.
        struct ParamReadCheck<'p> {
            param_ids: &'p HirIdSet,
            found: bool,
        }

        impl<'tcx> Visitor<'tcx> for ParamReadCheck<'_> {
            /// Records a match when a local path resolves to one of the
            /// tracked parameter `HirId`s.
            fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
                if self.found {
                    return;
                }
                if let hir::ExprKind::Path(hir::QPath::Resolved(None, path)) = expr.kind
                    && let hir::def::Res::Local(hir_id) = path.res
                    && self.param_ids.contains(&hir_id)
                {
                    // Skip if clamped: e.g. param.min(CONST)
                    // Check parent context — if this param read is the
                    // receiver of a .min() or .clamp() call, it's clamped.
                    self.found = true;
                }
                intravisit::walk_expr(self, expr);
            }
        }

        let mut checker = ParamReadCheck {
            param_ids: &self.param_ids,
            found: false,
        };
        for stmt in stmts {
            intravisit::walk_stmt(&mut checker, stmt);
            if checker.found {
                return true;
            }
        }
        false
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
    /// Flags a growth-method call on a container type inside a loop.
    ///
    /// The receiver type is matched against [`SOROBAN_CONTAINER_TYPES`] and
    /// the method name against [`BYTES_APPEND_METHODS`].  Only syntactic
    /// loops are considered; multi-call closures are not flagged here.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            let method_name = path_segment.ident.name.as_str();
            if !BYTES_APPEND_METHODS.contains(&method_name) {
                return;
            }

            let is_container = if let Some(adt_def) = try_get_adt_def(cx, receiver) {
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
fn is_valid_short_symbol(symbol_str: &str) -> bool {
    if symbol_str.len() > 9 || symbol_str.is_empty() {
        return false;
    }
    symbol_str
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// =======================================================================
// storage_write_without_read — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub STORAGE_WRITE_WITHOUT_READ,
    Warn,
    "storage write without a corresponding read"
}
/// Late pass backing [`STORAGE_WRITE_WITHOUT_READ`].
///
/// Flags `set` calls on storage accessors when the same key was not
/// previously read (via `get` or `has`) in the same function body.
/// Writing without prior knowledge of the stored value may indicate a
/// logic error or unnecessary overwrite that wastes budget.
pub struct StorageWriteWithoutRead;
rustc_session::impl_lint_pass!(StorageWriteWithoutRead => [STORAGE_WRITE_WITHOUT_READ]);

impl<'tcx> LateLintPass<'tcx> for StorageWriteWithoutRead {
    /// Visits a function body, collecting all storage reads and writes, and
    /// emits a diagnostic for every write whose key was not preceded by a
    /// read on the same receiver-key pair.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: rustc_hir::intravisit::FnKind<'tcx>,
        _: &'tcx hir::FnDecl<'tcx>,
        body: &'tcx hir::Body<'tcx>,
        _: rustc_span::Span,
        _: rustc_hir::def_id::LocalDefId,
    ) {
        /// Collects storage-read method calls (`get`, `has`) keyed by
        /// receiver-snippet and key-snippet for later cross-referencing.
        struct ReadVisitor<'a, 'tcx> {
            cx: &'a LateContext<'tcx>,
            reads: HashSet<(String, String)>,
        }

        impl<'a, 'tcx> Visitor<'tcx> for ReadVisitor<'a, 'tcx> {
            /// Records `(receiver_snippet, key_snippet)` for every `get` or
            /// `has` call on a [`SOROBAN_STORAGE_TYPES`] receiver.
            fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
                if let hir::ExprKind::MethodCall(path_segment, receiver, args, _span) = &expr.kind {
                    let is_storage = if let Some(adt_def) = try_get_adt_def(self.cx, receiver) {
                        matches_any_path(self.cx, adt_def.did(), SOROBAN_STORAGE_TYPES)
                    } else {
                        false
                    };

                    let method_name = path_segment.ident.name.as_str();
                    if is_storage
                        && (method_name == "get" || method_name == "has")
                        && !args.is_empty()
                    {
                        let receiver_snippet =
                            snippet_opt(self.cx, receiver.span).unwrap_or_default();
                        let key_snippet = snippet_opt(self.cx, args[0].span).unwrap_or_default();
                        self.reads.insert((receiver_snippet, key_snippet));
                    }
                }
                intravisit::walk_expr(self, expr);
            }
        }

        /// Collects storage-write method calls (`set`) with receiver,
        /// key, and span for later comparison against the read set.
        struct WriteVisitor<'a, 'tcx> {
            cx: &'a LateContext<'tcx>,
            writes: Vec<(String, String, rustc_span::Span)>,
        }

        impl<'a, 'tcx> Visitor<'tcx> for WriteVisitor<'a, 'tcx> {
            /// Records `(receiver_snippet, key_snippet, span)` for every
            /// `set` call on a [`SOROBAN_STORAGE_TYPES`] receiver.
            fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
                if let hir::ExprKind::MethodCall(path_segment, receiver, args, span) = &expr.kind {
                    let is_storage = if let Some(adt_def) = try_get_adt_def(self.cx, receiver) {
                        matches_any_path(self.cx, adt_def.did(), SOROBAN_STORAGE_TYPES)
                    } else {
                        false
                    };

                    if is_storage && path_segment.ident.name.as_str() == "set" && args.len() >= 2 {
                        let receiver_snippet =
                            snippet_opt(self.cx, receiver.span).unwrap_or_default();
                        let key_snippet = snippet_opt(self.cx, args[0].span).unwrap_or_default();
                        self.writes.push((receiver_snippet, key_snippet, *span));
                    }
                }
                intravisit::walk_expr(self, expr);
            }
        }

        let reads = HashSet::new();
        let writes = Vec::new();
        let mut read_visitor = ReadVisitor { cx, reads };
        read_visitor.visit_body(body);

        let mut write_visitor = WriteVisitor { cx, writes };
        write_visitor.visit_body(body);

        for (w_receiver, w_key, w_span) in &write_visitor.writes {
            let has_read = read_visitor
                .reads
                .contains(&(w_receiver.clone(), w_key.clone()));
            if !has_read {
                span_lint_and_help(
                    cx,
                    STORAGE_WRITE_WITHOUT_READ,
                    *w_span,
                    "storage write without a corresponding read",
                    None,
                    "consider reading the value before writing or using `.has()` to check existence",
                );
            }
        }
    }
}

// =======================================================================
// inefficient_bytes_concat — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub INEFFICIENT_BYTES_CONCAT,
    Warn,
    "inefficient bytes concatenation"
}
/// Late pass backing [`INEFFICIENT_BYTES_CONCAT`].
///
/// Flags `Bytes + Bytes` (or mixed `Bytes + T`) expressions inside a loop.
/// Each concatenation copies the entire left-hand buffer on the host side,
/// producing O(n²) cost when repeated iteratively.
pub struct InefficientBytesConcat;
rustc_session::impl_lint_pass!(InefficientBytesConcat => [INEFFICIENT_BYTES_CONCAT]);

impl<'tcx> LateLintPass<'tcx> for InefficientBytesConcat {
    /// Visits binary `+` expressions and checks whether at least one
    /// operand is a `soroban_sdk::Bytes` type and the expression sits
    /// inside a loop.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::Binary(op, lhs, rhs) = &expr.kind
            && let hir::BinOpKind::Add = op.node
        {
            let lhs_ty = cx.typeck_results().expr_ty(lhs);
            let rhs_ty = cx.typeck_results().expr_ty(rhs);
            let is_bytes = is_bytes_type(cx, lhs_ty) || is_bytes_type(cx, rhs_ty);
            let is_in_loop = enclosing_loop(cx, expr).is_some();
            if is_bytes && is_in_loop {
                span_lint_and_help(
                    cx,
                    INEFFICIENT_BYTES_CONCAT,
                    expr.span,
                    "inefficient bytes concatenation in a loop",
                    None,
                    "use a Vec<u8> buffer to accumulate bytes and convert to Bytes after the loop",
                );
            }
        }
    }
}

fn is_bytes_type<'tcx>(cx: &LateContext<'tcx>, ty: rustc_middle::ty::Ty<'tcx>) -> bool {
    if let Some(adt_def) = ty_adt_def(ty) {
        match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "Bytes"])
    } else {
        false
    }
}

/// Extracts the `AdtDef` from a `Ty`, without peeling references.
fn ty_adt_def<'tcx>(ty: rustc_middle::ty::Ty<'tcx>) -> Option<rustc_middle::ty::AdtDef<'tcx>> {
    if let rustc_middle::ty::Adt(adt_def, _) = ty.kind() {
        Some(adt_def)
    } else {
        None
    }
}

// =======================================================================
// map_insert_in_loop — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub MAP_INSERT_IN_LOOP,
    Warn,
    "Map::insert called inside a loop"
}
/// Late pass backing [`MAP_INSERT_IN_LOOP`].
///
/// Flags `Map::insert` calls inside a loop.  Repeated inserts to a Soroban
/// `Map` trigger host-side reallocation on each call; accumulating
/// mutations in a native `HashMap` and writing once after the loop is
/// cheaper.
pub struct MapInsertInLoop;
rustc_session::impl_lint_pass!(MapInsertInLoop => [MAP_INSERT_IN_LOOP]);

impl<'tcx> LateLintPass<'tcx> for MapInsertInLoop {
    /// Flags a `.insert()` method call whose receiver is a
    /// `soroban_sdk::Map` when it appears inside a syntactic loop.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = &expr.kind {
            if path_segment.ident.name.as_str() != "insert" {
                return;
            }

            let is_map = if let Some(adt_def) = try_get_adt_def(cx, receiver) {
                match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "Map"])
            } else {
                false
            };

            if is_map && enclosing_loop(cx, expr).is_some() {
                span_lint_and_help(
                    cx,
                    MAP_INSERT_IN_LOOP,
                    expr.span,
                    "Map::insert inside a loop is expensive",
                    None,
                    "accumulate mutations in memory first and write once after the loop",
                );
            }
        }
    }
}

// =======================================================================
// signature_verification_in_loop — Lint
// =======================================================================

/// Signature-verification and public-key-recovery methods on the `Crypto`
/// accessor. Each of these performs a full elliptic-curve check — one of the
/// most CPU-expensive host functions available — so unlike a cheap, constant
/// host call this cost cannot be hoisted out of the loop: every iteration
/// verifies a different signature. Repeating it per iteration is a
/// structural sign that signatures are being checked one at a time instead
/// of via a scheme that supports batch or aggregate verification.
const SIGNATURE_VERIFICATION_METHODS: &[&str] =
    &["ed25519_verify", "secp256k1_recover", "secp256r1_verify"];

rustc_session::declare_lint! {
    pub SIGNATURE_VERIFICATION_IN_LOOP,
    Warn,
    "signature verification performed inside a loop"
}
/// Late pass backing [`SIGNATURE_VERIFICATION_IN_LOOP`].
///
/// Flags signature-verification and public-key-recovery calls
/// (`ed25519_verify`, `secp256k1_recover`, `secp256r1_verify`) on the
/// `Crypto` accessor when they appear inside a loop.  Each call performs a
/// full elliptic-curve check — among the most expensive host functions —
/// so per-iteration verification is a structural sign that batch or
/// aggregate verification should be considered.
pub struct SignatureVerificationInLoop;
rustc_session::impl_lint_pass!(SignatureVerificationInLoop => [SIGNATURE_VERIFICATION_IN_LOOP]);

impl<'tcx> LateLintPass<'tcx> for SignatureVerificationInLoop {
    /// Flags a method call matching [`SIGNATURE_VERIFICATION_METHODS`] on a
    /// `soroban_sdk::crypto::Crypto` or `CryptoHazmat` receiver inside a
    /// syntactic loop.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind
            && SIGNATURE_VERIFICATION_METHODS.contains(&path_segment.ident.name.as_str())
        {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_crypto = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                let did = adt_def.did();
                match_soroban_def_path(cx, did, &["soroban_sdk", "crypto", "Crypto"])
                    || match_soroban_def_path(cx, did, &["soroban_sdk", "crypto", "CryptoHazmat"])
            } else {
                false
            };

            if is_crypto && enclosing_loop(cx, expr).is_some() {
                span_lint_and_help(
                    cx,
                    SIGNATURE_VERIFICATION_IN_LOOP,
                    expr.span,
                    "signature verification inside a loop",
                    None,
                    "each call re-runs an expensive elliptic-curve check; consider a signature \
                     scheme that supports batch or aggregate verification, or move per-item \
                     auth to the callee via a bulk entrypoint",
                );
            }
        }
    }
}

// =======================================================================
// vec_where_slice_could_be_used — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub STORAGE_KEY_CONSTRUCTION_IN_LOOP,
    Warn,
    "storage key constructed inside a loop body where it could be hoisted"
}
/// Late pass backing [`STORAGE_KEY_CONSTRUCTION_IN_LOOP`].
///
/// Flags `Symbol::new(&env, key)` calls inside a loop body when the key
/// argument is loop-invariant.  Each `Symbol::new` allocates through the
/// host; hoisting the construction before the loop avoids repeated
/// allocations.
pub struct StorageKeyConstructionInLoop;
rustc_session::impl_lint_pass!(StorageKeyConstructionInLoop => [STORAGE_KEY_CONSTRUCTION_IN_LOOP]);

impl<'tcx> LateLintPass<'tcx> for StorageKeyConstructionInLoop {
    /// Flags `Symbol::new(&env, ...)` calls inside a loop body when the key
    /// does not depend on the loop variable.
    ///
    /// `Symbol::new` allocates through the host on every call. When the key
    /// is loop-invariant, constructing it once before the loop and reusing
    /// the result avoids repeated host allocations.
    ///
    /// Key construction that depends on the loop variable is not flagged:
    /// that is genuine per-iteration work and hoisting would change behaviour.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        // Match `Symbol::new(&env, key)` calls — a two-argument call whose
        // callee resolves to `soroban_sdk::Symbol::new`.
        if let hir::ExprKind::Call(callee, args) = expr.kind
            && args.len() == 2
            && let hir::ExprKind::Path(ref qpath) = callee.kind
            && let Some(def_id) = cx.qpath_res(qpath, callee.hir_id).opt_def_id()
            && match_soroban_def_path(cx, def_id, &["soroban_sdk", "Symbol", "new"])
        {
            // Only fire inside a syntactic loop body.
            if let Some(loop_expr) = enclosing_loop(cx, expr) {
                // Only fire when the key does NOT depend on the loop state.
                // A key that reads the loop variable (e.g. `Symbol::new(&env,
                // &format!("key_{}", i))`) is genuine per-iteration work.
                if !depends_on_loop_state(cx, loop_expr, expr) {
                    span_lint_and_help(
                        cx,
                        STORAGE_KEY_CONSTRUCTION_IN_LOOP,
                        expr.span,
                        "storage key constructed inside a loop body",
                        None,
                        "hoist the key construction outside the loop to avoid repeated host allocations",
                    );
                }
            }
        }
    }
}

rustc_session::declare_lint! {
    pub VEC_WHERE_SLICE_COULD_BE_USED,
    Warn,
    "soroban_sdk::Vec passed by value where a native Rust slice would suffice"
}
/// Late pass backing [`VEC_WHERE_SLICE_COULD_BE_USED`].
///
/// Flags by-value `soroban_sdk::Vec` function parameters where a native
/// Rust slice (`&[T]`) would suffice.  Passing a host-backed `Vec` by value
/// incurs metered copying on every call; a slice reference avoids the
/// overhead when the parameter is only read, not mutated.
pub struct VecWhereSliceCouldBeUsed;
rustc_session::impl_lint_pass!(VecWhereSliceCouldBeUsed => [VEC_WHERE_SLICE_COULD_BE_USED]);

impl<'tcx> LateLintPass<'tcx> for VecWhereSliceCouldBeUsed {
    /// Inspects each function parameter and emits a diagnostic when the
    /// parameter is a by-value `soroban_sdk::Vec` that is never mutated in
    /// the function body.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: rustc_hir::intravisit::FnKind<'tcx>,
        _: &'tcx hir::FnDecl<'tcx>,
        body: &'tcx hir::Body<'tcx>,
        _: rustc_span::Span,
        _: rustc_hir::def_id::LocalDefId,
    ) {
        let mutated = mutated_variables(body.value, cx);

        for param in body.params {
            if let hir::PatKind::Binding(_, hir_id, _ident, _) = param.pat.kind {
                // The parameter type as seen by the type checker.
                let ty = cx.typeck_results().node_type(param.hir_id);
                let peeled = ty.peel_refs();

                // Only flag by-value parameters (not &Vec or &mut Vec).
                if peeled != ty {
                    continue;
                }

                let is_soroban_vec = if let rustc_middle::ty::Adt(adt_def, _) = peeled.kind() {
                    match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "Vec"])
                } else {
                    false
                };

                if !is_soroban_vec {
                    continue;
                }

                // If the Vec is mutated anywhere in the function body, it
                // genuinely needs ownership — skip.
                if let Some(ref mutated) = mutated
                    && mutated.contains(&hir_id)
                {
                    continue;
                }

                // Known gap: `mutated_variables` tracks explicit mutations
                // (e.g. `push_back`) but not moves (passing the Vec to
                // another function by value, or returning it). A function
                // that moves the Vec elsewhere genuinely consumes it and
                // should not be flagged, but today it will be. This is
                // acceptable for an initial implementation — the same
                // trade-off exists in other lints in this repository.
                span_lint_and_help(
                    cx,
                    VEC_WHERE_SLICE_COULD_BE_USED,
                    param.span,
                    "soroban_sdk::Vec parameter could be replaced with a native Rust slice",
                    None,
                    "consider using native Rust types (e.g. `&[T]`) instead of \
                     `soroban_sdk::Vec` for read-only access to reduce host-side operations",
                );
            }
        }
    }
}

// =======================================================================
// extend_ttl_in_loop — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub EXTEND_TTL_IN_LOOP,
    Warn,
    "extend_ttl called inside a loop"
}
pub struct ExtendTtlInLoop;
rustc_session::impl_lint_pass!(ExtendTtlInLoop => [EXTEND_TTL_IN_LOOP]);

impl<'tcx> LateLintPass<'tcx> for ExtendTtlInLoop {
    /// Flags a call to `extend_ttl` on instance, persistent, or temporary
    /// storage when the call site sits directly inside a loop body.
    ///
    /// Each `extend_ttl` call is its own metered host call that *also*
    /// writes ledger state (a rent payment — see
    /// `docs/cost_rationale.md`, "Ledger Space Rent"), so issuing one per
    /// iteration multiplies both costs by the iteration count.
    ///
    /// This is deliberately narrower than [`SOROBAN_STORAGE_IN_LOOP`]'s
    /// direct in-loop check: that pass only treats `get`/`has`/`set` as a
    /// storage access before checking the loop, so `extend_ttl` never
    /// reaches it and there is no risk of double-reporting the same call.
    /// This lint owns the `extend_ttl`-in-loop diagnostic.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind
            && path_segment.ident.name.as_str() == "extend_ttl"
        {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_storage = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                matches_any_path(cx, adt_def.did(), SOROBAN_STORAGE_TYPES)
            } else {
                false
            };

            if is_storage && enclosing_loop(cx, expr).is_some() {
                span_lint_and_help(
                    cx,
                    EXTEND_TTL_IN_LOOP,
                    expr.span,
                    "extend_ttl called inside a loop",
                    None,
                    "each extend_ttl call is a separate metered host call that also writes \
                     ledger state, so calling it once per iteration multiplies both costs by \
                     the iteration count; batch the extension by collecting the keys/entries \
                     first and making a single extend_ttl call after the loop (if the entries \
                     share one accessor, e.g. multiple keys under Persistent), or extend once \
                     with a threshold sized generously enough to cover the whole batch instead \
                     of refreshing per-entry per-iteration",
                );
            }
        }
    }
}

// =======================================================================
// linear_scan_in_loop — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub LINEAR_SCAN_IN_LOOP,
    Warn,
    "linear scan on collection inside a loop — O(n²) cost"
}
pub struct LinearScanInLoop;
rustc_session::impl_lint_pass!(LinearScanInLoop => [LINEAR_SCAN_IN_LOOP]);

impl<'tcx> LateLintPass<'tcx> for LinearScanInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            let method_name = path_segment.ident.name.as_str();
            if !LINEAR_SCAN_METHODS.contains(&method_name) {
                return;
            }

            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_collection = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                matches_any_path(cx, adt_def.did(), SOROBAN_COLLECTION_TYPES)
            } else {
                false
            };

            if is_collection
                && let Some(loop_expr) = enclosing_loop(cx, expr)
                && !depends_on_loop_state(cx, loop_expr, expr)
            {
                span_lint_and_help(
                    cx,
                    LINEAR_SCAN_IN_LOOP,
                    expr.span,
                    "linear scan on collection inside a loop — O(n²) cost",
                    None,
                    "consider building a Map lookup outside the loop for O(1) access",
                );
            }
        }
    }
}

// =======================================================================
// persistent_read_without_ttl_extension — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub PERSISTENT_READ_WITHOUT_TTL_EXTENSION,
    Warn,
    "persistent storage read without TTL extension — archival cost cliff"
}
pub struct PersistentReadWithoutTtlExtension;
rustc_session::impl_lint_pass!(PersistentReadWithoutTtlExtension => [PERSISTENT_READ_WITHOUT_TTL_EXTENSION]);

struct PersistentReadVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    reads: Vec<&'tcx hir::Expr<'tcx>>,
    extend_ttl_found: bool,
}

impl<'tcx> Visitor<'tcx> for PersistentReadVisitor<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _) = expr.kind {
            let method_name = path_segment.ident.name.as_str();
            let receiver_ty = self.cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                if match_soroban_def_path(self.cx, adt_def.did(), &["soroban_sdk", "storage", "Persistent"]) {
                    match method_name {
                        "get" | "has" => {
                            self.reads.push(expr);
                        }
                        "extend_ttl" => {
                            self.extend_ttl_found = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        intravisit::walk_expr(self, expr);
    }
}

impl<'tcx> LateLintPass<'tcx> for PersistentReadWithoutTtlExtension {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: intravisit::FnKind<'tcx>,
        _decl: &'tcx hir::FnDecl<'tcx>,
        body: &'tcx hir::Body<'tcx>,
        _span: rustc_span::Span,
        _hir_id: rustc_hir::HirId,
    ) {
        let mut visitor = PersistentReadVisitor {
            cx,
            reads: Vec::new(),
            extend_ttl_found: false,
        };
        visitor.visit_body(body);

        if visitor.extend_ttl_found || visitor.reads.is_empty() {
            return;
        }

        for read_expr in &visitor.reads {
            span_lint_and_help(
                cx,
                PERSISTENT_READ_WITHOUT_TTL_EXTENSION,
                read_expr.span,
                "persistent storage read without TTL extension — archival cost cliff",
                None,
                "after reading from persistent storage, call extend_ttl on the same key to avoid paying archival cost on subsequent access",
            );
        }
    }
}

// =======================================================================
// require_auth_in_loop — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub REQUIRE_AUTH_IN_LOOP,
    Warn,
    "Address::require_auth or require_auth_for_args called inside a loop"
}
pub struct RequireAuthInLoop;
rustc_session::impl_lint_pass!(RequireAuthInLoop => [REQUIRE_AUTH_IN_LOOP]);

const REQUIRE_AUTH_METHODS: &[&str] = &["require_auth", "require_auth_for_args"];

impl<'tcx> LateLintPass<'tcx> for RequireAuthInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind
            && REQUIRE_AUTH_METHODS.contains(&path_segment.ident.name.as_str())
        {
            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_address = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "Address"])
            } else {
                false
            };

            if is_address && enclosing_loop(cx, expr).is_some() {
                span_lint_and_help(
                    cx,
                    REQUIRE_AUTH_IN_LOOP,
                    expr.span,
                    "authorization call inside a loop",
                    None,
                    "collect distinct addresses first and authorize each once before the loop",
                );
            }
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

/// Benchmarks the read/write matching lookup backing
/// [`STORAGE_WRITE_WITHOUT_READ`] — a `HashSet::contains` lookup (current)
/// against the `Vec::iter().any()` scan it replaced — on synthetic data
/// sized to approximate a function body with many storage operations.
/// Not a correctness test beyond the parity assertion: mirrors
/// `cargo-cost-lint/benches/linter_performance.rs`'s `Instant`-based,
/// no-hard-assertion-on-timing reporting style, since this workspace has
/// no Criterion dependency.
#[test]
fn storage_write_without_read_lookup_benchmark() {
    const N: usize = 500;

    let reads: HashSet<(String, String)> = (0..N)
        .map(|i| (format!("storage_{}", i % 7), format!("key_{i}")))
        .collect();
    let writes: Vec<(String, String)> = (0..N)
        .map(|i| (format!("storage_{}", i % 7), format!("other_key_{i}")))
        .collect();
    let reads_vec: Vec<(String, String)> = reads.iter().cloned().collect();

    let started = std::time::Instant::now();
    let hashset_misses = writes
        .iter()
        .filter(|(receiver, key)| !reads.contains(&(receiver.clone(), key.clone())))
        .count();
    let hashset_elapsed = started.elapsed();

    let started = std::time::Instant::now();
    let vec_misses = writes
        .iter()
        .filter(|(w_receiver, w_key)| {
            !reads_vec
                .iter()
                .any(|(r_receiver, r_key)| r_receiver == w_receiver && r_key == w_key)
        })
        .count();
    let vec_elapsed = started.elapsed();

    assert_eq!(
        hashset_misses, vec_misses,
        "HashSet and Vec lookups must agree on which writes are missing a read"
    );

    eprintln!(
        "storage_write_without_read_lookup/{N}x{N}: hashset={hashset_elapsed:?} vec_scan={vec_elapsed:?}"
    );
}
