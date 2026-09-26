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

/// Late pass backing [`REQUIRE_AUTH_IN_LOOP`].
impl<'tcx> LateLintPass<'tcx> for RequireAuthInLoop {
    /// Flags a `require_auth` / `require_auth_for_args` call on a
    /// `soroban_sdk::Address` when the call sits inside a loop.
    ///
    /// The three conditions are checked in cost order, cheapest first: the
    /// method name is read straight off the segment, the receiver type needs a
    /// typeck lookup, and [`enclosing_loop`] walks the HIR to the surrounding
    /// loop expression only for the calls that already look like candidates.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind
            && is_require_auth_method(path_segment.ident.name.as_str())
            && is_address_receiver(cx, receiver)
            && enclosing_loop(cx, expr).is_some()
        {
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
