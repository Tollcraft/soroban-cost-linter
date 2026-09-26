//! Flags loops whose iteration count is set by the caller and whose body
//! performs a storage write.
//!
//! # Why this is a cost problem
//!
//! Soroban charges every invocation for the resources it consumes, and the
//! caller chooses the size of the `Vec`, `Map` or `Bytes` they hand to an
//! entrypoint. A loop bounded by such a value and containing a storage write
//! multiplies an attacker-controlled iteration count by a metered host call,
//! so one invocation can exhaust the budget — and the failure surfaces as a
//! revert, so the caller learns nothing.
//!
//! The remediation is to bound the loop (clamp or reject the input, or batch
//! the writes) before using it as a loop bound.
//!
//! # Detection
//!
//! `check_expr` fires for every expression in the crate, so the guards are
//! ordered cheapest-first and each one that fails returns immediately:
//!
//! 1. The expression must desugar to a `for` loop. `while` and `loop` are
//!    skipped — their bounds are not reliably recoverable, and flagging them
//!    produced false positives on deliberately bounded loops.
//! 2. The loop must iterate over caller-supplied input ([`iterates_over_input`]).
//! 3. The body must contain a storage write ([`contains_storage_write`]).
//!
//! Because step 1 hands us the loop body as part of the same
//! [`higher::ForLoop`] match, step 3 reuses it rather than re-running
//! `higher::ForLoop::hir` to recover the very same node.
//!
//! # A deliberate false-positive bias
//!
//! Both type tests match on a substring of the type's `Debug` name rather than
//! on an exact def path. That is intentional: see [`COLLECTION_FRAGMENTS`] and
//! [`STORAGE_TYPE_FRAGMENTS`]. It is also why the `Ty` is formatted once per
//! query and the resulting `String` is scanned in place, rather than being
//! re-rendered for each fragment tested.

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::higher;
use rustc_hir::intravisit::{Visitor, walk_expr};
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::UNBOUNDED_INPUT_LOOP;

declare_lint_pass!(UnboundedInputLoop => [UNBOUNDED_INPUT_LOOP]);

/// Storage-mutating methods. Each is a write to host-backed state, so one
/// call per iteration is one metered write per iteration.
const WRITE_METHODS: &[&str] = &["set", "put", "make_persistent"];

/// Methods that turn a collection into an iterator, covering both spellings of
/// `for x in input.iter()`.
const ITERATING_METHODS: &[&str] = &["iter", "iter_combined"];

/// Type-name fragments identifying a caller-sized collection.
///
/// Substring matching on the rendered type name is deliberate. An exact
/// def-path check would remove over-reporting on a non-Soroban `Vec`, but the
/// two failure modes are not symmetric: a false *negative* means the lint
/// silently stops firing, while a false *positive* costs a developer one
/// `#[allow]`. The bias is towards reporting.
const COLLECTION_FRAGMENTS: &[&str] = &["Vec", "Map", "Array", "Bytes", "Set"];

/// Type-name fragments identifying a Soroban storage accessor, matched
/// substring-wise for the same reason as [`COLLECTION_FRAGMENTS`].
const STORAGE_TYPE_FRAGMENTS: &[&str] = &["Storage", "Instance", "Persistent", "Temporary"];

/// Late pass backing [`UNBOUNDED_INPUT_LOOP`].
impl<'tcx> LateLintPass<'tcx> for UnboundedInputLoop {
    /// Reports a `for` loop that iterates over caller-supplied input and
    /// writes to storage in its body.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Only `for` loops over a caller-supplied collection are recognized
        // as input-driven iteration. `while`/`loop` forms are conservatively
        // skipped to avoid flagging intentionally bounded loops.
        //
        // `body` is carried out of this match so the write check below does
        // not have to desugar the loop a second time to recover it.
        let Some(higher::ForLoop {
            arg: iter, body, ..
        }) = higher::ForLoop::hir(expr)
        else {
            return;
        };

        if !iterates_over_input(cx, iter) {
            return;
        }

        if !contains_storage_write(cx, body) {
            return;
        }

        span_lint_and_help(
            cx,
            UNBOUNDED_INPUT_LOOP,
            expr.span,
            "loop is driven by caller-supplied input and performs storage writes \
             inside the loop body",
            None,
            "the number of iterations is controlled by the caller and each iteration \
             performs a storage write; bound the loop (e.g. cap iterations or batch the \
             writes) so a single invocation cannot exhaust the budget",
        );
    }
}

/// Returns `true` when `iter` yields values whose count depends on a
/// caller-supplied collection.
///
/// Each arm covers one way of spelling such a loop head:
///
/// - `input.iter()` / `input.iter_combined(..)` — an explicit iterator;
/// - `input` / `&input` — moving or borrowing the collection directly;
/// - `0..input.len()` — a range whose end comes from a collection.
fn iterates_over_input(cx: &LateContext<'_>, iter: &Expr<'_>) -> bool {
    match iter.kind {
        ExprKind::MethodCall(path, receiver, _args, _) => {
            // e.g. `for x in input.iter()` / `input.iter_combined(...)`
            ITERATING_METHODS.contains(&path.ident.as_str()) && is_collection(cx, receiver)
        }
        ExprKind::Path(QPath::Resolved(_, _)) | ExprKind::AddrOf(..) => {
            // e.g. `for x in &input` / `for x in input`
            is_collection(cx, iter)
        }
        ExprKind::Call(_, args) => {
            // e.g. `for x in 0..input.len()`. `any` short-circuits, so the
            // usual single-argument range costs one type lookup, not two.
            args.iter().any(|arg| is_input_driven(cx, arg))
        }
        _ => false,
    }
}

/// Whether `expr` is `.len()` on a caller-supplied collection, or is itself a
/// caller-supplied collection.
fn is_input_driven(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    match expr.kind {
        // `0..input.len()`: the bound is the length of a caller-sized value.
        ExprKind::MethodCall(path, receiver, _args, _) => {
            path.ident.as_str() == "len" && is_collection(cx, receiver)
        }
        _ => is_collection(cx, expr),
    }
}

/// Conservatively treats any value whose type name matches a known Soroban
/// collection name as being supplied by the caller.
///
/// The type is rendered once and the resulting `String` is scanned in place.
/// Note that no `ends_with` check is needed alongside `contains`: a
/// `contains` hit on `suffix` already implies the name ends with `suffix`, so
/// testing both would repeat half the work for no additional matches.
fn is_collection(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr).peel_refs();
    type_name_contains_any(ty, COLLECTION_FRAGMENTS)
}

/// Whether the rendered name of `ty` contains any of `fragments`.
///
/// This is the single allocation point of the two type tests in this module,
/// shared by [`is_collection`] and [`is_storage_receiver`]. The scan itself
/// lives in [`name_contains_any`], which is the part worth unit-testing.
fn type_name_contains_any(ty: rustc_middle::ty::Ty<'_>, fragments: &[&str]) -> bool {
    name_contains_any(&format!("{ty:?}"), fragments)
}

/// Whether `name` contains any of `fragments`.
///
/// Pure and allocation-free, so the matching rules can be tested directly
/// without constructing a `Ty` (which needs a `TyCtxt` and so is not
/// available to a plain `#[test]`).
fn name_contains_any(name: &str, fragments: &[&str]) -> bool {
    fragments.iter().any(|fragment| name.contains(fragment))
}

/// Returns `true` when the loop body `body` performs a storage write.
fn contains_storage_write<'tcx>(cx: &LateContext<'tcx>, body: &'tcx Expr<'tcx>) -> bool {
    let mut visitor = WriteVisitor { cx, found: false };
    visitor.visit_expr(body);
    visitor.found
}

/// Searches a loop body for a storage write, stopping at the first one found.
///
/// # Early exit
///
/// The caller only needs to know *whether* a write exists, and once one is
/// found no later sub-expression can change that answer. The walk therefore
/// stops there instead of descending into the rest of the body — which is
/// most of the body, in the common `storage.set(..)`-on-the-last-statement
/// shape.
///
/// The write method name is tested before the receiver type, so a method call
/// that is not a write at all is rejected without rendering a type name.
struct WriteVisitor<'v, 'tcx> {
    cx: &'v LateContext<'tcx>,
    found: bool,
}

impl<'v, 'tcx> Visitor<'tcx> for WriteVisitor<'v, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if self.found {
            return;
        }

        if let ExprKind::MethodCall(path, receiver, _args, _) = expr.kind
            && WRITE_METHODS.contains(&path.ident.as_str())
            && is_storage_receiver(self.cx, receiver)
        {
            self.found = true;
            return;
        }

        walk_expr(self, expr);
    }
}

/// Whether `expr` is typed as a Soroban storage accessor.
fn is_storage_receiver(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr);
    type_name_contains_any(ty, STORAGE_TYPE_FRAGMENTS)
}

#[cfg(test)]
mod tests {
    use super::{COLLECTION_FRAGMENTS, STORAGE_TYPE_FRAGMENTS, WRITE_METHODS, name_contains_any};

    /// An empty needle makes `str::contains` return `true` for every name,
    /// which would turn both type tests into a constant `true` and make the
    /// lint fire on every `for` loop containing any method call. This is the
    /// cheapest way to catch that class of edit.
    #[test]
    fn no_empty_match_fragments() {
        assert!(COLLECTION_FRAGMENTS.iter().all(|f| !f.is_empty()));
        assert!(STORAGE_TYPE_FRAGMENTS.iter().all(|f| !f.is_empty()));
    }

    /// The write methods are matched by exact name, so adding a non-mutating
    /// name here would start flagging read-only calls.
    #[test]
    fn write_methods_are_exact_storage_mutations() {
        assert_eq!(WRITE_METHODS, ["set", "put", "make_persistent"]);
        assert!(!WRITE_METHODS.contains(&"get"));
        assert!(!WRITE_METHODS.contains(&"has"));
    }

    /// An empty fragment list can never match. This documents the contract
    /// `is_collection` and `is_storage_receiver` rely on: an empty list is the
    /// "no opinion" case, not a match-everything case.
    #[test]
    fn empty_fragment_list_matches_nothing() {
        assert!(!name_contains_any("Instance", &[]));
    }

    /// A type that matches neither list must be rejected by both. `bool` and
    /// `u32` are the renderings of common non-Soroban types, and mistaking
    /// either for a collection or a storage accessor is the false positive
    /// this lint is most exposed to.
    #[test]
    fn unrelated_types_match_neither_list() {
        for name in ["bool", "u32", "i128", "()"] {
            assert!(
                !name_contains_any(name, COLLECTION_FRAGMENTS),
                "{name} must not look like a collection"
            );
            assert!(
                !name_contains_any(name, STORAGE_TYPE_FRAGMENTS),
                "{name} must not look like a storage accessor"
            );
        }
    }

    /// The positive half of the same contract: every Soroban collection and
    /// storage name the lint cares about must be recognised. Losing one of
    /// these is a silent false negative, which is the failure mode the
    /// substring heuristic is chosen to avoid.
    #[test]
    fn every_soroban_name_is_recognised() {
        for name in [
            "Vec",
            "Map",
            "Array",
            "Bytes",
            "Set",
            "Vec<soroban_sdk::map::Map<u32, u32>>",
        ] {
            assert!(
                name_contains_any(name, COLLECTION_FRAGMENTS),
                "{name} must be recognised as a collection"
            );
        }

        for name in [
            "Storage",
            "Instance",
            "Persistent",
            "Temporary",
            "soroban_sdk::storage::Persistent",
        ] {
            assert!(
                name_contains_any(name, STORAGE_TYPE_FRAGMENTS),
                "{name} must be recognised as a storage accessor"
            );
        }
    }

    /// A storage accessor name must not be mistaken for a collection, and vice
    /// versa. The two lists are used by different checks, and a name landing
    /// in both would make a storage read count as a caller-sized input.
    #[test]
    fn the_two_fragment_lists_are_disjoint() {
        for storage in STORAGE_TYPE_FRAGMENTS {
            assert!(
                !COLLECTION_FRAGMENTS.contains(storage),
                "{storage} appears in both fragment lists"
            );
        }
    }
}
