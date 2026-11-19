//! Flags a value that is shuttled through `Val` three or more times across a
//! binding chain within a single block.
//!
//! # Why this is a cost problem
//!
//! Every crossing of the Soroban native/`Val` boundary is a host call plus a
//! (de)serialisation. A chain such as `u32 -> Val -> VecU32 -> Val` carries one
//! logical value across that boundary three times, and each hop pays
//! `DispatchHostFunction` overhead plus the conversion work itself. Converting
//! once, directly to the representation the next step needs, moves the same
//! value in a single crossing.
//!
//! # Detection
//!
//! `check_block` walks the statements of one block in source order and
//! inspects each `let` initialiser — and the block's tail expression — for one
//! of the four `Val`-boundary conversion calls (`into_val`, `try_into_val`,
//! `from_val`, `try_from_val`), matched on the trait method's definition path
//! (see [`INTO_VAL`] and its siblings). Three rules keep the hop graph
//! honest:
//!
//! 1. **A hop must actually touch `Val`.** The blanket
//!    `impl<E, T, U> IntoVal<E, U> for T where T: Into<U>` performs a cheap,
//!    host-free native conversion; because neither side of such a call is
//!    `Val`, it is not a hop ([`touches_val`]).
//! 2. **Hops link through bindings.** A hop only extends a chain when its
//!    source is a local binding, followed through `.clone()` and recognised
//!    through `.unwrap()`/`.expect()` on the `try_*` forms ([`extend`]). Two
//!    independent value streams in the same block therefore never join into
//!    one chain.
//! 3. **One report per chain, at the terminal hop.** The whole chain's hop
//!    count is named in the message, so a four-hop tail reports four hops
//!    once, at the tail expression.
//!
//! Chains are per-block: nothing propagates across function boundaries, so a
//! helper whose signature demands a `Val` crossing contributes a single hop to
//! the caller's graph, exactly as the documented false-positive note describes.
//!
//! The threshold, [`VAL_CONVERSION_CHAIN_MIN_HOPS`], is the constant
//! `docs/lints/val_conversion_chain.md` documents; two conversions
//! (`T -> Val -> U`) are the minimal round trip a foreign API boundary demands
//! and belong to the sibling `redundant_val_conversion` lint.

use std::collections::{HashMap, HashSet};

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir as hir;
use rustc_hir::def::Res;
use rustc_hir::{Block, Expr, ExprKind, HirId, PatKind, QPath, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Ty;
use rustc_session::declare_lint_pass;
use rustc_span::{Span, sym};

use crate::{VAL_CONVERSION_CHAIN, match_soroban_def_path};

declare_lint_pass!(ValConversionChain => [VAL_CONVERSION_CHAIN]);

/// Minimum number of `Val`-boundary hops before a chain is flagged.
///
/// This is the threshold `docs/lints/val_conversion_chain.md` names; keep the
/// two in sync.
pub(crate) const VAL_CONVERSION_CHAIN_MIN_HOPS: usize = 3;

/// Def-path of `soroban_sdk::IntoVal::into_val`.
const INTO_VAL: &[&str] = &["soroban_sdk", "IntoVal", "into_val"];

/// Def-path of `soroban_sdk::TryIntoVal::try_into_val`.
const TRY_INTO_VAL: &[&str] = &["soroban_sdk", "TryIntoVal", "try_into_val"];

/// Def-path of `soroban_sdk::FromVal::from_val`, called in its
/// `T::from_val(env, val)` form.
const FROM_VAL: &[&str] = &["soroban_sdk", "FromVal", "from_val"];

/// Def-path of `soroban_sdk::TryFromVal::try_from_val`, called in its
/// `T::try_from_val(env, val)` form.
const TRY_FROM_VAL: &[&str] = &["soroban_sdk", "TryFromVal", "try_from_val"];

/// Def-path of `soroban_sdk::Val` — the type whose presence on either side of
/// a conversion is what makes it a boundary hop.
const VAL: &[&str] = &["soroban_sdk", "Val"];

/// One value's history through the binding chain of a single block.
#[derive(Clone, Copy)]
struct Chain {
    /// The binding the stream started from; identifies the value, so two
    /// streams never merge.
    root: HirId,
    /// Hops taken from `root` up to and including the conversion that produced
    /// the current binding.
    hops: usize,
    /// Span of the most recent conversion call — where a chain that ends here
    /// is reported.
    last_span: Span,
}

impl<'tcx> LateLintPass<'tcx> for ValConversionChain {
    /// Builds the hop graph of one block and reports every value that crossed
    /// the `Val` boundary [`VAL_CONVERSION_CHAIN_MIN_HOPS`] or more times.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        // Chains of the bindings produced by hops, keyed by the binding's
        // `HirId`, plus the set of bindings consumed as a hop's source and the
        // visit order for source-ordered reporting.
        let mut chains: HashMap<HirId, Chain> = HashMap::new();
        let mut consumed: HashSet<HirId> = HashSet::new();
        let mut order: Vec<HirId> = Vec::new();

        for stmt in block.stmts {
            if let StmtKind::Let(hir::LetStmt {
                pat,
                init: Some(init),
                ..
            }) = stmt.kind
                && let PatKind::Binding(_, binding, _, _) = pat.kind
                && let Some((prev, hop_span)) = hop_of(cx, init, &mut chains, &mut consumed)
            {
                chains.insert(
                    binding,
                    Chain {
                        root: prev.root,
                        hops: prev.hops + 1,
                        last_span: hop_span,
                    },
                );
                order.push(binding);
            }
        }

        // A chain is reported where it terminates: at the last binding that no
        // later hop consumes, or at the block's tail expression. The tail is
        // analysed first so the binding it consumes is not mistaken for a
        // terminal — otherwise a tail-extended chain would report twice, once
        // at its last binding and once at the tail. Every consumed binding is
        // skipped below and picked up by the hop that consumes it, so each
        // stream reports exactly once, with its full hop count.
        let tail_report = block
            .expr
            .and_then(|tail| hop_of(cx, tail, &mut chains, &mut consumed))
            .filter(|(prev, _)| prev.hops + 1 >= VAL_CONVERSION_CHAIN_MIN_HOPS)
            .map(|(prev, hop_span)| (prev.hops + 1, hop_span));

        let mut reports: Vec<(usize, Span)> = order
            .iter()
            .filter(|binding| !consumed.contains(binding))
            .filter_map(|binding| {
                let chain = chains[binding];
                (chain.hops >= VAL_CONVERSION_CHAIN_MIN_HOPS)
                    .then_some((chain.hops, chain.last_span))
            })
            .collect();

        // The tail expression is the last thing a block evaluates, so its
        // report keeps the source order the golden records.
        if let Some(tail_report) = tail_report {
            reports.push(tail_report);
        }

        for (hops, span) in reports {
            let message = chain_message(hops);
            span_lint_and_help(
                cx,
                VAL_CONVERSION_CHAIN,
                span,
                message,
                None,
                "the value is bounced through `Val` and back multiple times; convert it directly \
                 to the final representation instead of hopping through `Val` at each call site \
                 — every hop is a metered host call",
            );
        }
    }
}

/// Analyses one `let` initialiser or tail expression for a single
/// `Val`-boundary hop.
///
/// Returns the chain the hop extends (before the hop) and the span of the
/// conversion call itself — the span the lint reports when the chain ends
/// there. `None` means the expression is not a boundary hop of a trackable
/// local value.
fn hop_of<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    chains: &mut HashMap<HirId, Chain>,
    consumed: &mut HashSet<HirId>,
) -> Option<(Chain, Span)> {
    match expr.kind {
        // `v.try_into_val(env).unwrap()` / `.expect(..)`: the adapter only
        // unwraps the conversion's `Result`; the hop is the call inside.
        ExprKind::MethodCall(segment, receiver, _, _)
            if matches!(segment.ident.as_str(), "unwrap" | "expect") =>
        {
            hop_of(cx, receiver, chains, consumed)
        }
        // `value.into_val(env)` / `value.try_into_val(env)`.
        ExprKind::MethodCall(segment, receiver, _, _) => {
            let method = match segment.ident.as_str() {
                "into_val" => INTO_VAL,
                "try_into_val" => TRY_INTO_VAL,
                _ => return None,
            };
            let did = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
            if !match_soroban_def_path(cx, did, method) {
                return None;
            }
            let source = cx.typeck_results().expr_ty(receiver);
            let result = ok_ty(cx, cx.typeck_results().expr_ty(expr));
            if !touches_val(cx, source, result) {
                return None;
            }
            extend(receiver, chains, consumed).map(|chain| (chain, expr.span))
        }
        // `T::from_val(env, value)` / `T::try_from_val(env, value)`.
        ExprKind::Call(callee, args) => {
            let ExprKind::Path(ref qpath) = callee.kind else {
                return None;
            };
            let QPath::TypeRelative(_, segment) = qpath else {
                return None;
            };
            let method = match segment.ident.as_str() {
                "from_val" => FROM_VAL,
                "try_from_val" => TRY_FROM_VAL,
                _ => return None,
            };
            let did = cx.qpath_res(qpath, callee.hir_id).opt_def_id()?;
            if !match_soroban_def_path(cx, did, method) {
                return None;
            }
            let value = args.get(1)?;
            let source = cx.typeck_results().expr_ty(value);
            let result = ok_ty(cx, cx.typeck_results().expr_ty(expr));
            if !touches_val(cx, source, result) {
                return None;
            }
            extend(value, chains, consumed).map(|chain| (chain, expr.span))
        }
        _ => None,
    }
}

/// Resolves the chain a hop consumes, following through `.clone()`.
///
/// The receiver must bottom out in a local binding — a value produced by
/// anything else (a function call, a field access) has no trackable stream, so
/// the hop is not linkable and the chain starts afresh at the next binding.
fn extend<'tcx>(
    receiver: &'tcx Expr<'tcx>,
    chains: &HashMap<HirId, Chain>,
    consumed: &mut HashSet<HirId>,
) -> Option<Chain> {
    let mut expr = receiver;
    while let ExprKind::MethodCall(segment, inner, _, _) = expr.kind
        && segment.ident.as_str() == "clone"
    {
        expr = inner;
    }
    let ExprKind::Path(QPath::Resolved(None, path)) = expr.kind else {
        return None;
    };
    let Res::Local(binding) = path.res else {
        return None;
    };
    consumed.insert(binding);
    Some(chains.get(&binding).copied().unwrap_or(Chain {
        root: binding,
        hops: 0,
        last_span: expr.span,
    }))
}

/// Whether the conversion puts `Val` on either side.
///
/// The blanket native `IntoVal` (`T: Into<U>`) never involves `Val`, so a
/// native-only chain such as `u32 -> u64 -> u128` is not a boundary crossing
/// and never becomes a hop.
fn touches_val(cx: &LateContext<'_>, source: Ty<'_>, result: Ty<'_>) -> bool {
    is_val_ty(cx, source) || is_val_ty(cx, result)
}

/// Whether `ty` is `soroban_sdk::Val` (through references).
fn is_val_ty(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    match ty.peel_refs().kind() {
        rustc_middle::ty::Adt(def, _) => match_soroban_def_path(cx, def.did(), VAL),
        _ => false,
    }
}

/// The `T` out of a `Result<T, E>`, so the `try_*` conversions are examined
/// by the value they produce rather than by the `Result` wrapper.
fn ok_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
    if let Some(result_did) = cx.tcx.get_diagnostic_item(sym::Result)
        && let rustc_middle::ty::Adt(def, substs) = ty.kind()
        && def.did() == result_did
        && let Some(ok) = substs[0].as_type()
    {
        return ok;
    }
    ty
}

/// The warning's message; the wording is pinned by the ui golden.
fn chain_message(hops: usize) -> String {
    format!("conversion chain through `Val`: {hops} hops to move one value")
}

#[cfg(test)]
mod tests {
    use super::{
        FROM_VAL, INTO_VAL, TRY_FROM_VAL, TRY_INTO_VAL, VAL, VAL_CONVERSION_CHAIN_MIN_HOPS,
        chain_message,
    };

    /// The threshold is documented in `docs/lints/val_conversion_chain.md`;
    /// two hops belong to the sibling `redundant_val_conversion` lint.
    #[test]
    fn min_hops_matches_the_documented_threshold() {
        assert_eq!(VAL_CONVERSION_CHAIN_MIN_HOPS, 3);
    }

    /// Exactly the four `Val`-boundary trait methods are hops, matched by
    /// definition path. A typo here is a silent false negative on every
    /// real contract.
    #[test]
    fn the_four_hop_methods_are_the_boundary_conversions() {
        assert_eq!(INTO_VAL, ["soroban_sdk", "IntoVal", "into_val"]);
        assert_eq!(TRY_INTO_VAL, ["soroban_sdk", "TryIntoVal", "try_into_val"]);
        assert_eq!(FROM_VAL, ["soroban_sdk", "FromVal", "from_val"]);
        assert_eq!(TRY_FROM_VAL, ["soroban_sdk", "TryFromVal", "try_from_val"]);
    }

    /// Two trait methods sharing a name tail (e.g. `IntoVal::into_val` vs a
    /// hypothetical `NativeIntoVal::into_val`) must still be distinct paths.
    #[test]
    fn the_four_hop_paths_are_pairwise_distinct() {
        let all = [INTO_VAL, TRY_INTO_VAL, FROM_VAL, TRY_FROM_VAL];
        for (index, path) in all.iter().enumerate() {
            for other in &all[index + 1..] {
                assert_ne!(path, other, "hop paths must be distinct");
            }
        }
    }

    /// `Val` is what a hop must touch; matching by definition path applies the
    /// same rule to the ui fixture's mock module and the real SDK.
    #[test]
    fn val_path_is_pinned() {
        assert_eq!(VAL, ["soroban_sdk", "Val"]);
    }

    /// The ui golden pins this wording; the count is what tells the developer
    /// how many host calls the chain pays today.
    #[test]
    fn message_names_the_hop_count() {
        assert_eq!(
            chain_message(3),
            "conversion chain through `Val`: 3 hops to move one value"
        );
        assert_eq!(
            chain_message(4),
            "conversion chain through `Val`: 4 hops to move one value"
        );
    }
}
