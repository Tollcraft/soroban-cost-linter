//! `cross_contract_result_discarded` — flags a cross-contract invocation whose
//! return value is thrown away.
//!
//! `let _ = env.invoke_contract::<T>(..)` and a bare
//! `env.invoke_contract::<T>(..);` both pay for the whole call — the host
//! dispatches the callee, instantiates its VM context, meters its execution and
//! converts the return value back across the boundary — and then drop the
//! answer. When the return value is genuinely uninteresting the discard is
//! better spelled as a named binding (`let _result = ...`) so the intent is
//! visible and the lint stays quiet.
//!
//! Detection is purely structural:
//!
//! 1. the expression is a method call named [`INVOKE_METHODS`] whose receiver
//!    resolves to `soroban_sdk::Env`,
//! 2. the call's return type is not `()` (a unit result has nothing to
//!    discard), and
//! 3. the call's value is discarded — bound to the wildcard `_` pattern, or
//!    used as a statement with a trailing semicolon.
//!
//! The pass keys off the `soroban_sdk::Env` definition path, not off the
//! fixture's local mock module, so the same shape is recognized in real
//! contracts and in the UI fixtures.

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir as hir;
use rustc_hir::{Node, PatKind, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{CROSS_CONTRACT_RESULT_DISCARDED, is_type_match};

declare_lint_pass!(CrossContractResultDiscarded => [CROSS_CONTRACT_RESULT_DISCARDED]);

/// `soroban_sdk::Env`, the only receiver type whose invocations are tracked.
const SOROBAN_ENV_TYPE: &[&[&str]] = &[&["soroban_sdk", "Env"]];

/// Methods on `Env` that dispatch a cross-contract call. Both the infallible
/// and the fallible spelling pay the same dispatch cost, so both are tracked.
const INVOKE_METHODS: &[&str] = &["invoke_contract", "try_invoke_contract"];

/// Message reported for a discarded cross-contract call result.
const CROSS_CONTRACT_RESULT_DISCARDED_MSG: &str = "cross-contract call result discarded";

/// Help note attached to [`CROSS_CONTRACT_RESULT_DISCARDED_MSG`].
const CROSS_CONTRACT_RESULT_DISCARDED_HELP: &str = "a cross-contract invocation pays for a full host \
     dispatch, its own metered execution, and converting the return value back across the \
     boundary; if the call is made for its side effect and the return value is not needed, bind \
     it to a named variable (e.g. `let _result = ...`) or silence this lint with \
     `#[allow(cross_contract_result_discarded)]`";

/// Late pass backing [`CROSS_CONTRACT_RESULT_DISCARDED`].
impl<'tcx> LateLintPass<'tcx> for CrossContractResultDiscarded {
    /// Flags an `Env::invoke_contract` / `Env::try_invoke_contract` call whose
    /// non-unit result is discarded.
    ///
    /// The guards run cheapest first: the method name is read off the call
    /// segment, the receiver type needs one typeck lookup, the return type is
    /// already cached in the typeck results, and only then is the HIR parent
    /// consulted to see whether the value was thrown away.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind else {
            return;
        };
        if !INVOKE_METHODS.contains(&path_segment.ident.name.as_str()) {
            return;
        }
        if !is_type_match(cx, cx.typeck_results().expr_ty(receiver), SOROBAN_ENV_TYPE) {
            return;
        }
        // A `-> ()` call has no value to discard, so `let _ = ...` and the
        // statement form are both fine.
        if cx.typeck_results().expr_ty(expr).is_unit() {
            return;
        }
        if !is_result_discarded(cx, expr) {
            return;
        }

        span_lint_and_help(
            cx,
            CROSS_CONTRACT_RESULT_DISCARDED,
            expr.span,
            CROSS_CONTRACT_RESULT_DISCARDED_MSG,
            None,
            CROSS_CONTRACT_RESULT_DISCARDED_HELP,
        );
    }
}

/// Returns `true` when the value produced by `expr` is discarded.
///
/// Two shapes count:
///
/// - `let _ = expr;` — the call is bound to the wildcard pattern, so the value
///   can never be read. A named binding, including an `_`-prefixed one, is
///   deliberately kept out of scope: that is the documented way to say the
///   discard is intentional.
/// - `expr;` — the call is a statement whose value is dropped immediately.
fn is_result_discarded(cx: &LateContext<'_>, expr: &hir::Expr<'_>) -> bool {
    match cx.tcx.parent_hir_node(expr.hir_id) {
        Node::LetStmt(local) => matches!(local.pat.kind, PatKind::Wild),
        Node::Stmt(stmt) => matches!(stmt.kind, StmtKind::Semi(_)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{INVOKE_METHODS, SOROBAN_ENV_TYPE};

    /// The lint keys off method names, so a stray entry would start flagging
    /// unrelated calls. Pin the list to what the lint documents.
    #[test]
    fn methods_are_exactly_the_two_invocation_calls() {
        assert_eq!(INVOKE_METHODS, ["invoke_contract", "try_invoke_contract"]);
    }

    /// Only `soroban_sdk::Env` is tracked; keep the path table honest.
    #[test]
    fn only_env_is_tracked() {
        assert_eq!(SOROBAN_ENV_TYPE, [&["soroban_sdk", "Env"][..]]);
    }
}
