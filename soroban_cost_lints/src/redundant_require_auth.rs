use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::snippet_opt;
use rustc_hir::{self as hir, ExprKind, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use std::collections::HashMap;

use crate::REDUNDANT_REQUIRE_AUTH;

declare_lint_pass!(RedundantRequireAuth => [REDUNDANT_REQUIRE_AUTH]);

const REQUIRE_AUTH_METHODS: &[&str] = &["require_auth", "require_auth_for_args"];

impl<'tcx> LateLintPass<'tcx> for RedundantRequireAuth {
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx hir::Block<'tcx>) {
        // source-text -> span of first require_auth call on that address
        let mut first_auth: HashMap<String, rustc_span::Span> = HashMap::new();

        for stmt in block.stmts {
            let expr = match stmt.kind {
                StmtKind::Let(hir::LetStmt {
                    init: Some(init), ..
                }) => init,
                StmtKind::Expr(expr) | StmtKind::Semi(expr) => expr,
                _ => continue,
            };

            if let ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
                let method = path_segment.ident.name.as_str();

                // Cross-contract call resets authorization tracking.
                if (method == "invoke_contract" || method == "try_invoke_contract")
                    && is_env_receiver(cx, receiver)
                {
                    first_auth.clear();
                    continue;
                }

                // require_auth / require_auth_for_args on an Address.
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
                        first_auth.entry(key_text).or_insert(expr.span);
                    }
                }
            }
        }
    }
}

/// Returns `true` if `receiver` has type `soroban_sdk::Env`.
fn is_env_receiver<'tcx>(cx: &LateContext<'tcx>, receiver: &'tcx hir::Expr<'tcx>) -> bool {
    let peeled = cx.typeck_results().expr_ty(receiver).peel_refs();
    let ty_str = format!("{:?}", peeled);
    ty_str.contains("soroban_sdk::Env") || ty_str.contains("Env")
}

/// Returns `true` if `receiver` has type `soroban_sdk::Address`.
fn is_address_receiver<'tcx>(cx: &LateContext<'tcx>, receiver: &'tcx hir::Expr<'tcx>) -> bool {
    let peeled = cx.typeck_results().expr_ty(receiver).peel_refs();
    let ty_str = format!("{:?}", peeled);
    ty_str.contains("soroban_sdk::Address") || ty_str.contains("Address")
}
