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
//! 1. The expression must be a loop. `for` loops are matched through
//!    [`higher::ForLoop`]; `while` loops are checked through their condition
//!    (the block's tail expression), and a bare `loop` through the
//!    caller-derived condition guarding its `break`. A `loop` with no such
//!    exit condition is not caller-controlled and stays silent.
//! 2. The iteration must be driven by caller-supplied input ([`iterates_over_input`]
//!    for `for` loops, [`bound_is_caller_controlled`] for the other forms).
//! 3. The body must contain a storage write ([`contains_storage_write`] /
//!    [`block_contains_storage_write`]).
//!
//! Because step 1 hands us the loop body as part of the same
//! [`higher::ForLoop`] match, step 3 reuses it rather than re-running
//! `higher::ForLoop::hir` to recover the very same node.
//!
//! # Two type tests, two matching rules
//!
//! The collection test matches Soroban definition paths exactly —
//! [`SOROBAN_COLLECTION_TYPES`] plus the `Bytes` row in
//! [`SOROBAN_BYTES_TYPES`] — and the storage test keeps the older substring
//! scan over the type's `Debug` name ([`STORAGE_TYPE_FRAGMENTS`]).
//!
//! The split is deliberate. The collection half was a substring scan too, and
//! that made it unable to tell a caller-supplied `soroban_sdk::Map` from a map
//! the contract built itself: `std::collections::HashMap<u32, u32>` renders as
//! `HashMap<u32, u32>`, which contains `Map`, so the loop was reported as
//! caller-driven as soon as its body wrote. A `#[allow]` on that report silences
//! every other collection in the function too, so the false positive could not
//! be suppressed narrowly ([#1036]). An exact def path has the opposite bias — a
//! caller-supplied collection this table does not name is now a false *negative*
//! — so the table below lists each accepted type with the SDK module that
//! declares it.
//!
//! The storage half keeps its scan: the four accessor names do not occur inside
//! any `std` type a method receiver can have, so there was never an unsuppressible
//! false positive to remove there, and the check stays a single `format!` plus an
//! in-place scan rather than a `DefId` resolution.
//!
//! # What the table accepts, and what it drops
//!
//! Accepted, with the module that declares the type in soroban-sdk 26.1.1 and
//! 28.0.0:
//!
//! | type | declaration |
//! | ---- | ----------- |
//! | `Vec<T>` | `src/vec.rs:88` |
//! | `Map<K, V>` | `src/map.rs:103` |
//! | `Bytes` | `src/bytes.rs:252` |
//!
//! Dropped from the old fragment list: `Array` and `Set`. Neither exists —
//! `grep 'struct Array\|struct Set' src/` over 26.1.1, 28.0.0 and 29.0.1
//! returns nothing, and there is no `array.rs` or `set.rs` module — so those
//! two rows could only ever match a `std` or user type whose name happened to
//! contain them. They were false-positive generators, not coverage.

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::higher;
use rustc_hir::def::Res;
use rustc_hir::intravisit::{Visitor, walk_expr};
use rustc_hir::{BinOpKind, Expr, ExprKind, LoopSource, Node, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{SOROBAN_COLLECTION_TYPES, UNBOUNDED_INPUT_LOOP, is_type_match};

declare_lint_pass!(UnboundedInputLoop => [UNBOUNDED_INPUT_LOOP]);

/// Storage-mutating methods. Each is a write to host-backed state, so one
/// call per iteration is one metered write per iteration.
///
/// Checked against soroban-sdk 26.1.1 and 28.0.0 (`storage.rs`): `Persistent`,
/// `Temporary` and `Instance` all export `set`, `update`, `try_update` and
/// `remove`. The former `put` and `make_persistent` exist in neither:
/// `make_persistent` left the storage API before those versions.
const WRITE_METHODS: &[&str] = &["set", "update", "try_update", "remove"];

/// Methods that turn a collection into an iterator, covering the `iter()`
/// spelling plus the `Map` projections that yield caller-sized iteration.
///
/// Checked against soroban-sdk 26.1.1 and 28.0.0 (`vec.rs`, `map.rs`): `Vec`
/// exports `iter`, `Map` exports `iter`, `keys` and `values`. The former
/// `iter_combined` was dropped from `Map` and exists in neither version.
const ITERATING_METHODS: &[&str] = &["iter", "keys", "values"];

/// The `Bytes` row the collection test accepts on top of
/// [`SOROBAN_COLLECTION_TYPES`].
///
/// `Bytes` is a caller-sized collection in exactly the same way `Vec` and
/// `Map` are — `env` receives it as a serialized host object and its length is
/// chosen by the caller — but it is absent from `SOROBAN_COLLECTION_TYPES`,
/// which lists only the two types the linear-scan lint reasons about. The
/// module segment is `bytes` (`bytes.rs:252` in soroban-sdk 28.0.0), the same
/// shape as the `vec`/`map` rows.
const SOROBAN_BYTES_TYPES: &[&[&str]] = &[&["soroban_sdk", "bytes", "Bytes"]];

/// Type-name fragments identifying a Soroban storage accessor.
///
/// Deliberately still a substring scan: the four names sit at the end of the
/// accessor's def path (`soroban_sdk::storage::Persistent`, ...) and do not
/// occur inside any `std` type a method receiver can have, so there is no
/// unsuppressible false positive here to remove. See the module docs.
const STORAGE_TYPE_FRAGMENTS: &[&str] = &["Storage", "Instance", "Persistent", "Temporary"];

/// Message reported for a caller-driven loop that writes to storage.
const UNBOUNDED_INPUT_LOOP_MSG: &str = "loop is driven by caller-supplied input and performs storage writes \
     inside the loop body";

/// Help note attached to [`UNBOUNDED_INPUT_LOOP_MSG`].
const UNBOUNDED_INPUT_LOOP_HELP: &str = "the number of iterations is controlled by the caller and each iteration \
     performs a storage write; bound the loop (e.g. cap iterations or batch the \
     writes) so a single invocation cannot exhaust the budget";

/// Late pass backing [`UNBOUNDED_INPUT_LOOP`].
impl<'tcx> LateLintPass<'tcx> for UnboundedInputLoop {
    /// Reports a loop that is driven by caller-supplied input and writes to
    /// storage in its body. All three loop forms are recognized: `for` over a
    /// caller-supplied collection, `while` with a caller-derived condition,
    /// and a bare `loop` whose `break` is guarded by a caller-derived
    /// condition.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // `for` loops over a caller-supplied collection are recognized as
        // input-driven iteration.
        //
        // `body` is carried out of this match so the write check below does
        // not have to desugar the loop a second time to recover it.
        if let Some(higher::ForLoop {
            arg: iter, body, ..
        }) = higher::ForLoop::hir(expr)
        {
            if iterates_over_input(cx, iter) && contains_storage_write(cx, body) {
                report(cx, expr.span);
            }
            return;
        }

        let ExprKind::Loop(block, _label, source, _) = expr.kind else {
            return;
        };

        match source {
            // `while i < count`: the condition is the block's tail
            // expression; the statements are the body.
            LoopSource::While => {
                if let Some(cond) = block.expr
                    && bound_is_caller_controlled(cx, cond)
                    && block_contains_storage_write(cx, block)
                {
                    report(cx, expr.span);
                }
            }
            // A bare `loop` has no condition of its own; it is caller-driven
            // exactly when its exit is a `break` guarded by a caller-derived
            // comparison.
            LoopSource::Loop => {
                if exit_is_caller_controlled(cx, block) && block_contains_storage_write(cx, block) {
                    report(cx, expr.span);
                }
            }
            // Desugared `for` loops were handled by `higher::ForLoop` above.
            LoopSource::ForLoop => {}
        }
    }
}

/// Emits the lint at `span`.
fn report(cx: &LateContext<'_>, span: rustc_span::Span) {
    span_lint_and_help(
        cx,
        UNBOUNDED_INPUT_LOOP,
        span,
        UNBOUNDED_INPUT_LOOP_MSG,
        None,
        UNBOUNDED_INPUT_LOOP_HELP,
    );
}

/// Returns `true` when `iter` yields values whose count depends on a
/// caller-supplied collection.
///
/// Each arm covers one way of spelling such a loop head:
///
/// - `input.iter()` / `input.keys()` / `input.values()` — an explicit
///   iterator or `Map` projection; [`is_collection`] sees the receiver, so
///   `keys()`/`values()` on a caller-supplied `Map` match even though they
///   return a `Vec`;
/// - `input` / `&input` — moving or borrowing the collection directly;
/// - `0..input.len()` — a range whose end comes from a collection.
fn iterates_over_input(cx: &LateContext<'_>, iter: &Expr<'_>) -> bool {
    match iter.kind {
        ExprKind::MethodCall(path, receiver, _args, _) => {
            // e.g. `for x in input.iter()` / `input.keys()` / `input.values()`
            ITERATING_METHODS.contains(&path.ident.as_str()) && is_collection(cx, receiver)
        }
        ExprKind::Path(QPath::Resolved(_, _)) | ExprKind::AddrOf(..) => {
            // e.g. `for x in &input` / `for x in input`
            is_collection(cx, iter)
        }
        // A range literal `a..b` is lowered as a `Struct` expression for
        // `core::ops::range::Range` with `start`/`end` fields — it is *not* a
        // `Call`, which is why this arm looks for `Struct` and inspects each
        // field expression ([#788]: `0..input.len()` and `0..count` previously
        // never reached the bound checks at all).
        ExprKind::Struct(_, fields, _) => {
            fields.iter().any(|field| is_caller_bound(cx, field.expr))
        }
        // `for x in (0..a, 0..b)`-style tuples are not reachable today but
        // the same field-walk argument covers them if HIR shape changes.
        ExprKind::Tup(parts) => parts.iter().any(|part| is_caller_bound(cx, part)),
        // `a..=b` lowers to a `RangeInclusive::new(a, b)` call.
        ExprKind::Call(_, args) => args.iter().any(|arg| is_caller_bound(cx, arg)),
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

/// Whether a `while` condition or a `break` guard is controlled by the
/// caller: a numeric comparison (`<`, `<=`, `>`, `>=`) with at least one side
/// derived from caller input.
fn bound_is_caller_controlled(cx: &LateContext<'_>, cond: &Expr<'_>) -> bool {
    match cond.kind {
        ExprKind::Binary(op, lhs, rhs) => {
            matches!(
                op.node,
                BinOpKind::Lt | BinOpKind::Le | BinOpKind::Gt | BinOpKind::Ge
            ) && (is_caller_bound(cx, lhs) || is_caller_bound(cx, rhs))
        }
        _ => false,
    }
}

/// Whether one side of a loop-bound comparison is caller-controlled: either
/// a caller-supplied collection (or its `.len()`, reusing [`is_input_driven`]),
/// or an unvalidated contract parameter.
fn is_caller_bound(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    is_input_driven(cx, expr) || is_unvalidated_parameter(cx, expr)
}

/// Whether `expr` is a function parameter, read directly.
///
/// A parameter's binding pattern hangs directly off the function's parameter
/// list, so its parent node is a [`Node::Param`]. A local introduced by `let`
/// — including a validated or clamped copy of a parameter — has a
/// `Node::LetStmt` parent instead and does not match. That is the distinction
/// this lint relies on to stay silent on validated bounds.
fn is_unvalidated_parameter(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ExprKind::Path(QPath::Resolved(None, path)) = expr.kind else {
        return false;
    };
    let Res::Local(binding) = path.res else {
        return false;
    };
    matches!(cx.tcx.parent_hir_node(binding), Node::Param(_))
}

/// Whether `expr` is typed as one of the caller-sized Soroban collections.
///
/// An exact definition-path match against [`SOROBAN_COLLECTION_TYPES`] and
/// [`SOROBAN_BYTES_TYPES`], so a `std::collections::HashMap` — which shares
/// the `Map` suffix that the previous substring scan keyed on — is not
/// mistaken for a `soroban_sdk::Map`. `is_type_match` peels references
/// itself; doing it here as well would only repeat the work.
fn is_collection(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr);
    is_type_match(cx, ty, SOROBAN_COLLECTION_TYPES) || is_type_match(cx, ty, SOROBAN_BYTES_TYPES)
}

/// Whether the rendered name of `ty` contains any of `fragments`.
///
/// This is the single allocation point of the storage test, and the only
/// remaining user of the name scan in this module: [`is_storage_receiver`]
/// needs it because a storage accessor's `Debug` name is all that is
/// available without resolving a `DefId`. The scan itself lives in
/// [`name_contains_any`], which is the part worth unit-testing.
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

/// Returns `true` when any statement of `block` performs a storage write.
///
/// Used for `while` and `loop` bodies, which arrive as a `Block` whose tail
/// expression is the condition or a trailing value rather than loop body
/// content, so only the statements are scanned.
fn block_contains_storage_write<'tcx>(
    cx: &LateContext<'tcx>,
    block: &'tcx rustc_hir::Block<'tcx>,
) -> bool {
    block.stmts.iter().any(|stmt| {
        let mut visitor = WriteVisitor { cx, found: false };
        visitor.visit_stmt(stmt);
        visitor.found
    })
}

/// Whether a bare `loop` exits through a `break` whose guard is controlled by
/// the caller, e.g. `if i >= count { break; }` with `count` a parameter.
fn exit_is_caller_controlled<'tcx>(
    cx: &LateContext<'tcx>,
    block: &'tcx rustc_hir::Block<'tcx>,
) -> bool {
    let mut finder = ControlledBreakFinder { cx, found: false };
    finder.visit_block(block);
    finder.found
}

/// Searches a loop body for a caller-derived `if` guarding a `break`,
/// stopping at the first one found.
///
/// A `break` inside a nested loop breaks *that* loop, not the one being
/// examined, so the walk does not descend into nested `Loop` expressions.
struct ControlledBreakFinder<'v, 'tcx> {
    cx: &'v LateContext<'tcx>,
    found: bool,
}

impl<'v, 'tcx> Visitor<'tcx> for ControlledBreakFinder<'v, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if self.found {
            return;
        }

        if matches!(expr.kind, ExprKind::Loop(..)) {
            return;
        }

        if let ExprKind::If(cond, then_block, _) = expr.kind
            && bound_is_caller_controlled(self.cx, cond)
            && block_has_unlabeled_break(then_block)
        {
            self.found = true;
            return;
        }

        walk_expr(self, expr);
    }
}

/// Whether the `then` side of a caller-derived `if` contains an (unlabeled)
/// `break` that would exit the surrounding loop. Nested loops are skipped for
/// the same reason as in [`ControlledBreakFinder`].
fn block_has_unlabeled_break<'tcx>(then: &'tcx Expr<'tcx>) -> bool {
    let mut finder = UnlabeledBreakFinder { found: false };
    finder.visit_expr(then);
    finder.found
}

struct UnlabeledBreakFinder {
    found: bool,
}

impl<'hir> Visitor<'hir> for UnlabeledBreakFinder {
    fn visit_expr(&mut self, expr: &'hir Expr<'hir>) {
        if self.found {
            return;
        }

        if matches!(expr.kind, ExprKind::Loop(..)) {
            return;
        }

        if let ExprKind::Break(destination, _) = expr.kind
            && destination.label.is_none()
        {
            self.found = true;
            return;
        }

        walk_expr(self, expr);
    }
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
    use super::{
        ITERATING_METHODS, SOROBAN_BYTES_TYPES, STORAGE_TYPE_FRAGMENTS, WRITE_METHODS,
        name_contains_any,
    };
    use crate::SOROBAN_COLLECTION_TYPES;

    /// An empty needle makes `str::contains` return `true` for every name,
    /// which would turn the storage test into a constant `true` and make the
    /// lint fire on every `for` loop containing any method call. This is the
    /// cheapest way to catch that class of edit.
    #[test]
    fn no_empty_match_fragments() {
        assert!(STORAGE_TYPE_FRAGMENTS.iter().all(|f| !f.is_empty()));
    }

    /// The write methods are matched by exact name, so adding a non-mutating
    /// name here would start flagging read-only calls.
    #[test]
    fn write_methods_are_exact_storage_mutations() {
        assert_eq!(WRITE_METHODS, ["set", "update", "try_update", "remove"]);
        assert!(!WRITE_METHODS.contains(&"get"));
        assert!(!WRITE_METHODS.contains(&"has"));
    }

    /// Every name must be exported by the soroban-sdk this crate builds
    /// against (checked against 26.1.1 and 28.0.0: `iter` on `Vec`/`Map`,
    /// `keys`/`values` on `Map`). A stale entry like the former
    /// `iter_combined` silently disables half the detection surface.
    #[test]
    fn iterating_methods_are_sdk_iterator_spellings() {
        assert_eq!(ITERATING_METHODS, ["iter", "keys", "values"]);
        assert!(!ITERATING_METHODS.contains(&"iter_combined"));
    }

    /// An empty fragment list can never match. This documents the contract
    /// `is_collection` and `is_storage_receiver` rely on: an empty list is the
    /// "no opinion" case, not a match-everything case.
    #[test]
    fn empty_fragment_list_matches_nothing() {
        assert!(!name_contains_any("Instance", &[]));
    }

    /// A type that matches the storage scan must be a Soroban storage
    /// accessor. `bool` and `u32` are the renderings of common non-Soroban
    /// types, and the `std` collection names are here because mistaking one
    /// of those for a storage accessor would re-open the false positive the
    /// collection half used to have.
    #[test]
    fn unrelated_types_are_not_storage_accessors() {
        for name in [
            "bool",
            "u32",
            "i128",
            "()",
            "HashMap<u32, u32>",
            "HashSet<u32>",
        ] {
            assert!(
                !name_contains_any(name, STORAGE_TYPE_FRAGMENTS),
                "{name} must not look like a storage accessor"
            );
        }
    }

    /// The positive half of the storage contract: every accessor name the lint
    /// cares about must be recognised. Losing one is a silent false negative,
    /// which is the failure mode the substring heuristic is chosen to avoid.
    #[test]
    fn every_soroban_storage_name_is_recognised() {
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

    /// Every accepted collection row must be rooted at `soroban_sdk` and name
    /// the declaring module. A single-segment row is how the old
    /// `["Vec", "Map", "Array", "Bytes", "Set"]` list reached
    /// `std::collections::HashMap`: the rendered name contains `Map`, so the
    /// scan matched a type the lint has no opinion about. Requiring the
    /// `soroban_sdk` root and a module segment makes that shape unrepresentable.
    #[test]
    fn collection_paths_are_soroban_rooted_and_qualified() {
        for path in SOROBAN_COLLECTION_TYPES.iter().chain(SOROBAN_BYTES_TYPES) {
            assert_eq!(path.first().copied(), Some("soroban_sdk"), "{path:?}");
            assert!(path.len() >= 2, "{path:?} must name a module");
            assert!(path.iter().all(|segment| !segment.is_empty()), "{path:?}");
        }
    }

    /// The collection table is exactly `Vec`, `Map`, `Bytes`. `Array` and
    /// `Set` must not come back: soroban-sdk 26.1.1, 28.0.0 and 29.0.1 declare
    /// neither, so those rows could only ever match a non-Soroban type.
    #[test]
    fn collection_types_are_exactly_vec_map_bytes() {
        let mut typed: Vec<&str> = SOROBAN_COLLECTION_TYPES
            .iter()
            .chain(SOROBAN_BYTES_TYPES)
            .map(|path| *path.last().expect("each row ends in a type name"))
            .collect();
        typed.sort_unstable();
        assert_eq!(typed, ["Bytes", "Map", "Vec"]);
        assert!(!typed.contains(&"Array"));
        assert!(!typed.contains(&"Set"));
    }

    /// A storage accessor name must not be mistaken for a collection name, and
    /// vice versa. The two are used by different checks, and a name landing in
    /// both would make a storage read count as a caller-sized input.
    #[test]
    fn storage_fragments_and_collection_types_are_disjoint() {
        for storage in STORAGE_TYPE_FRAGMENTS {
            assert!(
                SOROBAN_COLLECTION_TYPES
                    .iter()
                    .chain(SOROBAN_BYTES_TYPES)
                    .all(|path| !path.contains(storage)),
                "{storage} appears in both the storage fragments and the collection paths"
            );
        }
    }
}
