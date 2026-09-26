//! Flags `unwrap`/`expect` applied directly to a Soroban storage read.
//!
//! A storage read is metered the moment it is issued, and it returns `None`
//! whenever the key has never been written or its TTL has lapsed. Unwrapping
//! that `Option` therefore does not just panic: the contract has already been
//! charged for the read and for everything the invocation did before it, and
//! the caller receives nothing in return. Handling the `None` case explicitly
//! turns the same read into a recoverable error path.
//!
//! The pass is deliberately narrow — it reports *only* a panic-on-`None` that
//! is written directly on a `get` call. See [`UnwrapOnStorageGet::check_expr`]
//! for the exact scope and the deliberately excluded shapes.

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::is_in_test;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{SOROBAN_STORAGE_TYPES, UNWRAP_ON_STORAGE_GET, is_type_match};

declare_lint_pass!(UnwrapOnStorageGet => [UNWRAP_ON_STORAGE_GET]);

/// Methods that turn a `None` into a panic. `expect` is included because it
/// is the same trap with a caller-supplied message: the read's `Option` is
/// consumed unconditionally either way.
const PANICKING_METHODS: &[&str] = &["unwrap", "expect"];

/// The accessor method whose `Option` result must not be unwrapped.
const STORAGE_READ_METHOD: &str = "get";

/// Message reported for a panic-on-`None` over a storage read.
const UNWRAP_ON_STORAGE_GET_MSG: &str =
    "unwrap on a storage read traps the contract when the key is missing or expired";

/// Help note attached to [`UNWRAP_ON_STORAGE_GET_MSG`].
const UNWRAP_ON_STORAGE_GET_HELP: &str = "handle the None case explicitly with \
     unwrap_or, unwrap_or_else, or an early return carrying a proper error — \
     work already metered before the trap is charged to the caller while \
     delivering nothing";

/// Late pass backing [`UNWRAP_ON_STORAGE_GET`].
impl<'tcx> LateLintPass<'tcx> for UnwrapOnStorageGet {
    /// Flags `.unwrap()` / `.expect()` whose receiver is a `get` call on one
    /// of [`SOROBAN_STORAGE_TYPES`] (`Storage`, `Instance`, `Persistent`,
    /// `Temporary`).
    ///
    /// # Scope
    ///
    /// Only an unwrap written *directly* on a storage read is reported. The
    /// following are intentionally left alone:
    ///
    /// - a read whose `Option` is consumed by `unwrap_or` / `unwrap_or_else`,
    ///   or consumed by a `match` — these all handle the missing-key case;
    /// - an `unwrap` on any other `Option`/`Result`, which is out of scope for
    ///   a cost lint and is Clippy's `unwrap_used` territory;
    /// - anything under `#[cfg(test)]` or inside a test module, detected with
    ///   `clippy_utils::is_in_test`. Unwrapping in a test is idiomatic: the
    ///   test author controls the fixture and a failing assertion is the
    ///   desired outcome, so a panic costs no metered budget.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(read_receiver) = panicking_unwrap_receiver(expr) else {
            return;
        };

        // Test code is exempt: an unwrap there is an assertion, not a
        // metered-on-failure code path.
        if is_in_test(cx.tcx, expr.hir_id) {
            return;
        }

        if !is_storage_read_receiver(cx, read_receiver) {
            return;
        }

        span_lint_and_help(
            cx,
            UNWRAP_ON_STORAGE_GET,
            expr.span,
            UNWRAP_ON_STORAGE_GET_MSG,
            None,
            UNWRAP_ON_STORAGE_GET_HELP,
        );
    }
}

/// If `expr` is a call to a panicking method (`unwrap` / `expect`), returns
/// the receiver expression, i.e. the value whose `None` would be unwrapped.
///
/// Every other expression — including a call to any other method — yields
/// `None`. The receiver is returned rather than a `bool` so the caller can
/// keep the two conditions (is it a panic, is it on a storage read) separate
/// and independently testable.
fn panicking_unwrap_receiver<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
        return PANICKING_METHODS
            .contains(&path_segment.ident.name.as_str())
            .then_some(receiver);
    }
    None
}

/// Whether `receiver` is the accessor of a Soroban storage read, i.e. a
/// `get` call made on one of the [`SOROBAN_STORAGE_TYPES`].
///
/// References are peeled before the type is inspected, so this matches both
/// `instance.get(&k)` and a `get` through a `&Instance` binding.
fn is_storage_read_receiver<'tcx>(cx: &LateContext<'tcx>, receiver: &'tcx Expr<'tcx>) -> bool {
    let ExprKind::MethodCall(get_segment, storage_receiver, _get_args, _get_span) = receiver.kind
    else {
        return false;
    };

    get_segment.ident.name.as_str() == STORAGE_READ_METHOD
        && is_type_match(
            cx,
            cx.typeck_results().expr_ty(storage_receiver),
            SOROBAN_STORAGE_TYPES,
        )
}

#[cfg(test)]
mod tests {
    use super::{PANICKING_METHODS, STORAGE_READ_METHOD};

    /// The lint keys off method *names*, so a name that is accidentally added
    /// to the panic list would start flagging unrelated code. These assertions
    /// pin the two lists down to the methods the lint documents.
    #[test]
    fn panic_list_is_exactly_unwrap_and_expect() {
        assert_eq!(PANICKING_METHODS, ["unwrap", "expect"]);
    }

    #[test]
    fn storage_read_method_is_get() {
        assert_eq!(STORAGE_READ_METHOD, "get");
    }
}
