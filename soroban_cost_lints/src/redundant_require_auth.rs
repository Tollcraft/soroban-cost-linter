//! `redundant_require_auth` — flags a second `require_auth` on an address in
//! the same function.
//!
//! Authorizing an address is cumulative: once `require_auth` (or
//! `require_auth_for_args`) succeeds for an address, that address stays
//! authorized for the rest of the invocation. A second call on the same address
//! therefore adds no security and only pays the host-call cost again.
//!
//! # How tracking works
//!
//! The pass walks a block's top-level statements in order and keeps a map from
//! the address's *source text* to the span of the first authorization seen for
//! it. Using source text as the key means two syntactically identical
//! expressions are treated as the same address even when they are distinct
//! bindings; see [`is_address_receiver`].
//!
//! A cross-contract call (`invoke_contract` / `try_invoke_contract` on `Env`)
//! resets the map: authorization does not survive into a callee's context in a
//! way this lint can reason about, so trusting it past that point would risk
//! false positives. The reasoning for the reset lives on the call site in
//! `check_block`.

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::snippet_opt;
use rustc_hir::{self as hir, ExprKind, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use std::collections::HashMap;

use crate::REDUNDANT_REQUIRE_AUTH;

declare_lint_pass!(RedundantRequireAuth => [REDUNDANT_REQUIRE_AUTH]);

/// Address authorization methods that are equivalent for this lint's purpose.
/// `require_auth_for_args` authorizes the same address for a specific argument
/// set, so a second authorization of that address is just as redundant.
const REQUIRE_AUTH_METHODS: &[&str] = &["require_auth", "require_auth_for_args"];

/// `Env` methods that perform a cross-contract call and therefore reset the
/// authorization tracking for the rest of the block.
const CROSS_CONTRACT_CALL_METHODS: &[&str] = &["invoke_contract", "try_invoke_contract"];

/// Late pass backing [`REDUNDANT_REQUIRE_AUTH`].
impl<'tcx> LateLintPass<'tcx> for RedundantRequireAuth {
    /// Flags an address whose authorization was already requested earlier in
    /// the same block.
    ///
    /// Only top-level statements are inspected, and only the call expression of
    /// each statement: a `require_auth` nested inside a larger expression is out
    /// of scope, which keeps the pass a single linear scan with no dataflow.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx hir::Block<'tcx>) {
        // Address source text -> span of the first authorization call on it.
        let mut first_auth: HashMap<String, rustc_span::Span> = HashMap::new();

        for stmt in block.stmts {
            // Only the expression a statement evaluates matters; patterns,
            // types, and let-without-init statements are irrelevant here.
            let expr = match stmt.kind {
                StmtKind::Let(hir::LetStmt {
                    init: Some(init), ..
                }) => init,
                StmtKind::Expr(expr) | StmtKind::Semi(expr) => expr,
                _ => continue,
            };

            if let ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
                let method = path_segment.ident.name.as_str();

                // A cross-contract call may re-enter code that authorizes
                // differently, and the outer authorization no longer cleanly
                // covers the later calls, so stop trusting prior entries.
                if CROSS_CONTRACT_CALL_METHODS.contains(&method) && is_env_receiver(cx, receiver) {
                    first_auth.clear();
                    continue;
                }

                // `require_auth` / `require_auth_for_args` on an `Address`. The
                // address is keyed by its printed source text.
                if REQUIRE_AUTH_METHODS.contains(&method)
                    && is_address_receiver(cx, receiver)
                    && let Some(key_text) = snippet_opt(cx, receiver.span)
                {
                    if let Some(&_prev_span) = first_auth.get(&key_text) {
                        span_lint_and_help(
                            cx,
                            REDUNDANT_REQUIRE_AUTH,
                            expr.span,
                            "require_auth already called on this address in this function",
                            None,
                            "remove this duplicate authorization call; the first require_auth on an address already establishes authorization for the entire invocation",
                        );
                    } else {
                        // `entry(..).or_insert(..)` keeps the first span, but a
                        // plain insert would be equivalent here since the key
                        // is only absent on this branch.
                        first_auth.entry(key_text).or_insert(expr.span);
                    }
                }
            }
        }
    }
}

/// Returns `true` if `receiver` has type `soroban_sdk::Env`.
///
/// The type is matched via its `Debug` rendering rather than the shared
/// `match_soroban_def_path` helper because the fixture SDKs in `ui/` do not
/// carry the same definition paths as the real SDK; the substring check keeps
/// the lint working in both. References are peeled first so `&Env` matches too.
fn is_env_receiver<'tcx>(cx: &LateContext<'tcx>, receiver: &'tcx hir::Expr<'tcx>) -> bool {
    let peeled = cx.typeck_results().expr_ty(receiver).peel_refs();
    let ty_str = format!("{:?}", peeled);
    ty_str.contains("soroban_sdk::Env") || ty_str.contains("Env")
}

/// Returns `true` if `receiver` has type `soroban_sdk::Address`.
///
/// Like [`is_env_receiver`], this uses the `Debug` rendering so it matches both
/// the real SDK and the UI fixtures. References are peeled first.
fn is_address_receiver<'tcx>(cx: &LateContext<'tcx>, receiver: &'tcx hir::Expr<'tcx>) -> bool {
    let peeled = cx.typeck_results().expr_ty(receiver).peel_refs();
    let ty_str = format!("{:?}", peeled);
    ty_str.contains("soroban_sdk::Address") || ty_str.contains("Address")
}
