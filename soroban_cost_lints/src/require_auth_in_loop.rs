//! `require_auth_in_loop` — flags authorization calls inside a loop.
//!
//! `Address::require_auth` and `require_auth_for_args` ask the host to verify
//! that the caller authorized the invocation. The first successful call for an
//! address establishes authorization for the whole invocation, so repeating it
//! on every iteration of a loop does no extra work but is metered each time.
//! Collecting the distinct addresses first and authorizing each once before the
//! loop is cheaper.
//!
//! The pass is structural: it does not check whether the address actually
//! varies between iterations, so authorizing a genuinely different address each
//! iteration is a known false positive. That case is pinned in
//! `soroban_cost_lints/ui/require_auth_in_loop.rs` so a future change to the
//! analysis has to make a deliberate decision about it.
//!
//! Call syntax
//! -----------
//!
//! Both the method-call spelling (`addr.require_auth()`) and the UFCS spelling
//! (`Address::require_auth(addr)`) are recognized. The method-call form is the
//! common one, but the UFCS form is valid Soroban, so a loop over
//! `Address::require_auth(some_addr)` must still be flagged.

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{REQUIRE_AUTH_IN_LOOP, enclosing_loop, is_type_match};

declare_lint_pass!(RequireAuthInLoop => [REQUIRE_AUTH_IN_LOOP]);

/// `soroban_sdk::Address`, the only receiver type whose authorization calls are
/// tracked. Authorization is per address, so a call on any other type is out of
/// scope.
const SOROBAN_ADDRESS_TYPE: &[&[&str]] = &[&["soroban_sdk", "Address"]];

/// Methods on `soroban_sdk::Address` that establish authorization. Both are
/// tracked: `require_auth_for_args` authorizes the same address for a specific
/// argument set, and repeating it in a loop is just as avoidable.
const REQUIRE_AUTH_METHODS: &[&str] = &["require_auth", "require_auth_for_args"];

/// Message reported for an authorization call sitting inside a loop.
const REQUIRE_AUTH_IN_LOOP_MSG: &str = "authorization call inside a loop";

/// Help note attached to [`REQUIRE_AUTH_IN_LOOP_MSG`].
const REQUIRE_AUTH_IN_LOOP_HELP: &str =
    "collect distinct addresses first and authorize each once before the loop";

/// Whether `method` is one of the authorization calls this lint tracks.
fn is_require_auth_method(method: &str) -> bool {
    REQUIRE_AUTH_METHODS.contains(&method)
}

/// Whether `receiver` resolves to a `soroban_sdk::Address` (references peeled).
fn is_address_receiver<'tcx>(cx: &LateContext<'tcx>, receiver: &'tcx hir::Expr<'tcx>) -> bool {
    is_type_match(
        cx,
        cx.typeck_results().expr_ty(receiver),
        SOROBAN_ADDRESS_TYPE,
    )
}

/// The method name a callee path segment carries, if it carries one.
///
/// Covers both path shapes a call to an associated function can take:
/// `QPath::Resolved` for a fully-resolvable `Address::require_auth`, and
/// `QPath::TypeRelative` for the desugaring of the same spelling.
fn qpath_method_name<'a>(qpath: &'a hir::QPath<'a>) -> Option<&'a str> {
    match qpath {
        hir::QPath::Resolved(_, path) => path.segments.last().map(|seg| seg.ident.name.as_str()),
        hir::QPath::TypeRelative(_, seg) => Some(seg.ident.name.as_str()),
    }
}

/// Whether `callee`/`args` form a UFCS authorization call on an address.
///
/// `Address::require_auth(addr)` desugars to `ExprKind::Call(path, [addr, ..])`:
/// the method name lives in the callee path, and the address being authorized
/// is the first argument.
fn is_ufcs_require_auth_call<'tcx>(
    cx: &LateContext<'tcx>,
    callee: &'tcx hir::Expr<'tcx>,
    args: &'tcx [hir::Expr<'tcx>],
) -> bool {
    let hir::ExprKind::Path(ref qpath) = callee.kind else {
        return false;
    };
    let Some(method) = qpath_method_name(qpath) else {
        return false;
    };
    if !is_require_auth_method(method) {
        return false;
    }
    // UFCS requires the address as the first argument.
    let Some(receiver) = args.first() else {
        return false;
    };
    is_address_receiver(cx, receiver)
}

/// Late pass backing [`REQUIRE_AUTH_IN_LOOP`].
impl<'tcx> LateLintPass<'tcx> for RequireAuthInLoop {
    /// Flags a `require_auth` / `require_auth_for_args` call on a
    /// `soroban_sdk::Address` when the call sits inside a loop.
    ///
    /// Both spellings are recognized:
    /// - method-call: `addr.require_auth()` / `addr.require_auth_for_args(..)`
    /// - UFCS: `Address::require_auth(addr)` / `Address::require_auth_for_args(addr, ..)`
    ///
    /// The conditions are checked in cost order, cheapest first: the method name
    /// is read straight off the segment (or path), the receiver type needs a
    /// typeck lookup, and [`enclosing_loop`] walks the HIR to the surrounding
    /// loop expression only for the calls that already look like candidates.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        let is_auth_call = match expr.kind {
            hir::ExprKind::MethodCall(path_segment, receiver, _, _) => {
                is_require_auth_method(path_segment.ident.name.as_str())
                    && is_address_receiver(cx, receiver)
            }
            hir::ExprKind::Call(callee, args) => is_ufcs_require_auth_call(cx, callee, args),
            _ => false,
        };
        if !is_auth_call {
            return;
        }
        if enclosing_loop(cx, expr).is_none() {
            return;
        }
        span_lint_and_help(
            cx,
            REQUIRE_AUTH_IN_LOOP,
            expr.span,
            REQUIRE_AUTH_IN_LOOP_MSG,
            None,
            REQUIRE_AUTH_IN_LOOP_HELP,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{REQUIRE_AUTH_METHODS, SOROBAN_ADDRESS_TYPE, is_require_auth_method};

    /// The lint keys off method names, so a name added by accident would start
    /// flagging unrelated calls. Pin the list to what the lint documents.
    #[test]
    fn methods_are_exactly_the_two_authorization_calls() {
        assert_eq!(
            REQUIRE_AUTH_METHODS,
            ["require_auth", "require_auth_for_args"]
        );
        assert!(is_require_auth_method("require_auth"));
        assert!(is_require_auth_method("require_auth_for_args"));
        assert!(!is_require_auth_method("require_auth_for_args_extra"));
        assert!(!is_require_auth_method("clone"));
    }

    /// Only `soroban_sdk::Address` is tracked; keep the path table honest.
    #[test]
    fn only_address_is_tracked() {
        assert_eq!(SOROBAN_ADDRESS_TYPE, [&["soroban_sdk", "Address"][..]]);
    }
}
