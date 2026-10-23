//! `bytes_slice_copy_in_loop` — flags `Bytes` slice/copy calls repeated on
//! every iteration of a loop.
//!
//! `Bytes::slice(start, end)` returns a fresh host object: the requested range
//! is copied into a new buffer and billed for. `copy_to_slice` and
//! `copy_from_slice` move the whole range in one direction each time they run.
//! A parser that steps through a payload four bytes at a time therefore copies
//! an average of half the remaining buffer per iteration — O(n²) bytes of
//! metered host traffic for what should be a linear scan.
//!
//! # Detection
//!
//! A method call named `slice`, `copy_to_slice` or `copy_from_slice`
//! ([`BYTES_SLICE_COPY_METHODS`]) whose receiver type is `soroban_sdk::Bytes`
//! ([`BYTES_TYPES`]) is reported when [`enclosing_loop`] returns a loop.
//!
//! The loop test uses the crate's [`enclosing_loop`] helper rather than
//! `enclosing_loop_or_closure`: the shipped `bytes_append_in_loop` semantics
//! are loop-only, and a closure body that the loop merely calls is not
//! necessarily executed once per iteration. The ui fixture encodes the same
//! split, so a closure-only call is not reported.
//!
//! The analysis deliberately does **not** require the slice bounds or the
//! destination buffer to be provably loop-dependent. Every one of these calls
//! re-copies its range when it runs, so a call sitting in a loop body pays that
//! copy per iteration regardless of where its arguments come from — the
//! fixture's `copy_from_slice(&[0u8, 1, 2, 3])` case, whose argument is a
//! constant, is exactly that shape and is expected to warn. Small, fixed-bound
//! loops are documented as `#[allow]`-able in the lint page rather than guessed
//! at statically.

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir as hir;
use rustc_hir::ExprKind;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{BYTES_SLICE_COPY_IN_LOOP, enclosing_loop, is_type_match};

declare_lint_pass!(BytesSliceCopyInLoop => [BYTES_SLICE_COPY_IN_LOOP]);

/// `Bytes` methods that allocate and copy a range of the buffer.
const BYTES_SLICE_COPY_METHODS: &[&str] = &["slice", "copy_to_slice", "copy_from_slice"];

/// The receiver type this lint targets: `soroban_sdk::Bytes` only, so native
/// `[u8]`/`Vec<u8>` helper code and the other SDK containers stay silent.
const BYTES_TYPES: &[&[&str]] = &[&["soroban_sdk", "Bytes"]];

/// Message reported for a slice/copy call inside a loop.
const BYTES_SLICE_COPY_IN_LOOP_MSG: &str =
    "repeatedly copying a sub-slice of `Bytes` inside a loop";

/// Help note attached to [`BYTES_SLICE_COPY_IN_LOOP_MSG`]; the wording is
/// pinned by the ui golden.
const BYTES_SLICE_COPY_IN_LOOP_HELP: &str = "prefer index-based access (e.g. `Bytes::get`) for \
     per-byte reads, or take the full slice once and iterate it outside the loop; only slice once, \
     not on every iteration";

impl<'tcx> LateLintPass<'tcx> for BytesSliceCopyInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        let ExprKind::MethodCall(segment, receiver, _args, _) = expr.kind else {
            return;
        };
        if !BYTES_SLICE_COPY_METHODS.contains(&segment.ident.as_str()) {
            return;
        }
        if !is_type_match(cx, cx.typeck_results().expr_ty(receiver), BYTES_TYPES) {
            return;
        }
        if enclosing_loop(cx, expr).is_none() {
            return;
        }
        span_lint_and_help(
            cx,
            BYTES_SLICE_COPY_IN_LOOP,
            expr.span,
            BYTES_SLICE_COPY_IN_LOOP_MSG,
            None,
            BYTES_SLICE_COPY_IN_LOOP_HELP,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{BYTES_SLICE_COPY_IN_LOOP_HELP, BYTES_SLICE_COPY_METHODS, BYTES_TYPES};

    /// The fixture pins exactly these three methods; adding a growth method
    /// here would duplicate `bytes_append_in_loop`.
    #[test]
    fn the_three_copy_methods_are_the_detected_ones() {
        assert_eq!(
            BYTES_SLICE_COPY_METHODS,
            ["slice", "copy_to_slice", "copy_from_slice"]
        );
    }

    /// Only `soroban_sdk::Bytes` is targeted — not `Vec`, `Map` or `[u8]`.
    #[test]
    fn only_sdk_bytes_is_matched() {
        assert_eq!(BYTES_TYPES, [["soroban_sdk", "Bytes"]]);
    }

    /// The ui golden pins this wording.
    #[test]
    fn help_text_is_pinned() {
        assert!(BYTES_SLICE_COPY_IN_LOOP_HELP.contains("Bytes::get"));
        assert!(BYTES_SLICE_COPY_IN_LOOP_HELP.contains("only slice once"));
    }
}
