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
use clippy_utils::res::MaybeResPath;
use clippy_utils::source::snippet_opt;
use clippy_utils::ty::peel_and_count_ty_refs;
use clippy_utils::usage::local_used_after_expr;
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

/// Whether `expr` sits inside something the runtime will execute more than
/// once: a syntactic loop, **or** a multi-call closure (`for_each`,
/// `Iterator::map` argument, etc.).
///
/// `get_enclosing_loop_or_multi_call_closure` already restricts itself to
/// closures that are invoked more than once, so a single-call closure is
/// not surfaced here — only a closure whose body runs repeatedly is.
///
/// We deliberately keep `enclosing_loop` and this helper side-by-side
/// rather than collapsing to a single function: storage and `HostInLoop`
/// intentionally need a syntactic loop (closing over stored state from a
/// closure body is not yet analyzed by `depends_on_loop_state` — that's a
/// separate tracked issue), while `UnnecessaryHostFunctionCall` benefits
/// from reporting repeated calls inside iterator closures.
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
        lint: STORAGE_WRITE_WITHOUT_READ,
        category: LintCategory::StorageOperations,
    },
    LintMetadata {
        lint: INEFFICIENT_BYTES_CONCAT,
        category: LintCategory::Memory,
    },
    LintMetadata {
        lint: MAP_INSERT_IN_LOOP,
        category: LintCategory::StorageOperations,
    },
    LintMetadata {
        lint: BYTES_APPEND_IN_LOOP,
        category: LintCategory::Memory,
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
        STORAGE_WRITE_WITHOUT_READ,
        INEFFICIENT_BYTES_CONCAT,
        MAP_INSERT_IN_LOOP,
        BYTES_APPEND_IN_LOOP,
    ]);
    lint_store.register_late_pass(|_| Box::new(SorobanStorageInLoop));
    lint_store.register_late_pass(|_| Box::new(RedundantEnvClone));
    lint_store.register_late_pass(|_| Box::new(UnnecessaryHostFunctionCall));
    lint_store.register_late_pass(|_| Box::new(HostInLoop));
    lint_store.register_late_pass(|_| Box::new(SymbolNewForShortLiteral));
    lint_store.register_late_pass(|_| Box::new(StorageWriteWithoutRead));
    lint_store.register_late_pass(|_| Box::new(InefficientBytesConcat));
    lint_store.register_late_pass(|_| Box::new(MapInsertInLoop));
    lint_store.register_late_pass(|_| Box::new(BytesAppendInLoop));
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

            if is_storage_access && enclosing_loop(cx, expr).is_some() {
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
        let receiver = if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) =
            expr.kind
            && path_segment.ident.name.as_str() == "clone"
        {
            receiver
        } else if let hir::ExprKind::Call(callee, args) = expr.kind
            && args.len() == 1
            && let hir::ExprKind::Path(ref qpath) = callee.kind
            && let Some(def_id) = cx.qpath_res(qpath, callee.hir_id).opt_def_id()
            && (match_soroban_def_path(cx, def_id, &["Clone", "clone"])
                || match_soroban_def_path(cx, def_id, &["Env", "clone"]))
        {
            &args[0]
        } else {
            return;
        };

        let receiver_ty = cx.typeck_results().expr_ty(receiver);
        let peeled_ty = receiver_ty.peel_refs();

        let is_env = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
            match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "Env"])
        } else {
            false
        };

        if is_env {
            // Clone on &Env produces an owned Env from a reference — genuinely needed.
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

// =======================================================================
// bytes_append_in_loop — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub BYTES_APPEND_IN_LOOP,
    Warn,
    "repeatedly growing SDK containers inside loops"
}
pub struct BytesAppendInLoop;
rustc_session::impl_lint_pass!(BytesAppendInLoop => [BYTES_APPEND_IN_LOOP]);

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

// =======================================================================
// storage_write_without_read — Lint
// =======================================================================

rustc_session::declare_lint! {
    pub STORAGE_WRITE_WITHOUT_READ,
    Warn,
    "storage write without a corresponding read"
}
pub struct StorageWriteWithoutRead;
rustc_session::impl_lint_pass!(StorageWriteWithoutRead => [STORAGE_WRITE_WITHOUT_READ]);

impl<'tcx> LateLintPass<'tcx> for StorageWriteWithoutRead {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: rustc_hir::intravisit::FnKind<'tcx>,
        _: &'tcx hir::FnDecl<'tcx>,
        body: &'tcx hir::Body<'tcx>,
        _: rustc_span::Span,
        _: rustc_hir::def_id::LocalDefId,
    ) {
        struct ReadVisitor<'a, 'tcx> {
            cx: &'a LateContext<'tcx>,
            reads: Vec<(String, String)>,
        }

        impl<'a, 'tcx> Visitor<'tcx> for ReadVisitor<'a, 'tcx> {
            fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
                if let hir::ExprKind::MethodCall(path_segment, receiver, args, _span) = &expr.kind {
                    let receiver_ty = self.cx.typeck_results().expr_ty(receiver);
                    let peeled_ty = receiver_ty.peel_refs();

                    let is_storage = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
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
                        self.reads.push((receiver_snippet, key_snippet));
                    }
                }
                intravisit::walk_expr(self, expr);
            }
        }

        struct WriteVisitor<'a, 'tcx> {
            cx: &'a LateContext<'tcx>,
            writes: Vec<(String, String, rustc_span::Span)>,
        }

        impl<'a, 'tcx> Visitor<'tcx> for WriteVisitor<'a, 'tcx> {
            fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
                if let hir::ExprKind::MethodCall(path_segment, receiver, args, span) = &expr.kind {
                    let receiver_ty = self.cx.typeck_results().expr_ty(receiver);
                    let peeled_ty = receiver_ty.peel_refs();

                    let is_storage = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
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

        let reads = Vec::new();
        let writes = Vec::new();
        let mut read_visitor = ReadVisitor { cx, reads };
        read_visitor.visit_body(body);

        let mut write_visitor = WriteVisitor { cx, writes };
        write_visitor.visit_body(body);

        for (w_receiver, w_key, w_span) in &write_visitor.writes {
            let has_read = read_visitor
                .reads
                .iter()
                .any(|(r_receiver, r_key)| r_receiver == w_receiver && r_key == w_key);
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
pub struct InefficientBytesConcat;
rustc_session::impl_lint_pass!(InefficientBytesConcat => [INEFFICIENT_BYTES_CONCAT]);

impl<'tcx> LateLintPass<'tcx> for InefficientBytesConcat {
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
    if let rustc_middle::ty::Adt(adt_def, _) = ty.kind() {
        match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "Bytes"])
    } else {
        false
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
pub struct MapInsertInLoop;
rustc_session::impl_lint_pass!(MapInsertInLoop => [MAP_INSERT_IN_LOOP]);

impl<'tcx> LateLintPass<'tcx> for MapInsertInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = &expr.kind {
            if path_segment.ident.name.as_str() != "insert" {
                return;
            }

            let receiver_ty = cx.typeck_results().expr_ty(receiver);
            let peeled_ty = receiver_ty.peel_refs();

            let is_map = if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
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

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
