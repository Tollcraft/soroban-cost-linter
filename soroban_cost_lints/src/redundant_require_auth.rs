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
//! The pass visits the statements of the enclosing function body in source
//! order, at every nesting level, and keeps one map from the address's *source
//! text* to the span of the first authorization seen for it. Using source text
//! as the key means two syntactically identical expressions are treated as the
//! same address even when they are distinct bindings; see
//! [`is_address_receiver`].
//!
//! The map lives for exactly one function: `check_fn` creates it fresh, so
//! duplicates in sibling blocks of one function are reported while two
//! authorizations in two different functions are not, and recursion cannot
//! leak a set into another invocation. Closures are separate function bodies
//! and get their own map — the outer walk does not descend into them, because
//! a closure may never be called and trusting its statements from the outside
//! would risk false positives.
//!
//! A cross-contract call (`invoke_contract` / `try_invoke_contract` on `Env`)
//! resets the map: authorization does not survive into a callee's context in a
//! way this lint can reason about, so trusting it past that point would risk
//! false positives. The reasoning for the reset lives on the call site in
//! [`AuthVisitor::check_stmt_expr`].

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::snippet_opt;
use rustc_hir::intravisit::{Visitor, walk_expr, walk_stmt};
use rustc_hir::{self as hir, ExprKind, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::def_id::LocalDefId;
use std::collections::HashMap;

use crate::{REDUNDANT_REQUIRE_AUTH, match_soroban_def_path, ty_adt_def};

declare_lint_pass!(RedundantRequireAuth => [REDUNDANT_REQUIRE_AUTH]);

/// Address authorization methods that are equivalent for this lint's purpose.
/// `require_auth_for_args` authorizes the same address for a specific argument
/// set, so a second authorization of that address is just as redundant.
const REQUIRE_AUTH_METHODS: &[&str] = &["require_auth", "require_auth_for_args"];

/// `Env` methods that perform a cross-contract call and therefore reset the
/// authorization tracking for the rest of the function.
const CROSS_CONTRACT_CALL_METHODS: &[&str] = &["invoke_contract", "try_invoke_contract"];

/// Late pass backing [`REDUNDANT_REQUIRE_AUTH`].
impl<'tcx> LateLintPass<'tcx> for RedundantRequireAuth {
    /// Flags addresses authorized twice anywhere within one function body.
    ///
    /// The seen-set is created here, per function, and handed to a visitor
    /// that walks the body's statements in source order at every nesting
    /// level. Keeping the map out of the pass struct means recursion and
    /// sibling functions can never see each other's entries.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: rustc_hir::intravisit::FnKind<'tcx>,
        _decl: &'tcx hir::FnDecl<'tcx>,
        body: &'tcx hir::Body<'tcx>,
        _span: rustc_span::Span,
        _id: LocalDefId,
    ) {
        // Address source text -> span of the first authorization call on it.
        let mut first_auth: HashMap<String, rustc_span::Span> = HashMap::new();
        let mut visitor = AuthVisitor {
            cx,
            first_auth: &mut first_auth,
        };
        visitor.visit_expr(body.value);
    }
}

/// Walks one function body, tracking authorizations in a caller-provided map.
struct AuthVisitor<'v, 'tcx> {
    cx: &'v LateContext<'tcx>,
    first_auth: &'v mut HashMap<String, rustc_span::Span>,
}

impl<'v, 'tcx> AuthVisitor<'v, 'tcx> {
    /// Applies the authorization-tracking rule to a statement's own
    /// expression — the only shape the pass inspects. A `require_auth` nested
    /// inside a larger expression is out of scope, which keeps the pass a
    /// single linear scan with no dataflow.
    fn check_stmt_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        let ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind else {
            return;
        };
        let method = path_segment.ident.name.as_str();

        // A cross-contract call may re-enter code that authorizes
        // differently, and the outer authorization no longer cleanly
        // covers the later calls, so stop trusting prior entries.
        if CROSS_CONTRACT_CALL_METHODS.contains(&method) && is_env_receiver(self.cx, receiver) {
            self.first_auth.clear();
            return;
        }

        // `require_auth` / `require_auth_for_args` on an `Address`. The
        // address is keyed by its printed source text.
        if REQUIRE_AUTH_METHODS.contains(&method)
            && is_address_receiver(self.cx, receiver)
            && let Some(key_text) = snippet_opt(self.cx, receiver.span)
        {
            if let Some(&_prev_span) = self.first_auth.get(&key_text) {
                span_lint_and_help(
                    self.cx,
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
                self.first_auth.entry(key_text).or_insert(expr.span);
            }
        }
    }
}

impl<'v, 'tcx> Visitor<'tcx> for AuthVisitor<'v, 'tcx> {
    fn visit_stmt(&mut self, stmt: &'tcx hir::Stmt<'tcx>) {
        // Only the expression a statement evaluates matters; patterns,
        // types, and let-without-init statements are irrelevant here.
        match stmt.kind {
            StmtKind::Let(hir::LetStmt {
                init: Some(init), ..
            }) => self.check_stmt_expr(init),
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => self.check_stmt_expr(expr),
            _ => {}
        }
        // Descend so statements of nested blocks (if arms, loop bodies,
        // standalone blocks) are visited in source order too.
        walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        // A closure is its own function body with its own map; it also may
        // never be called, so its statements must not influence the outer
        // function's tracking.
        if matches!(expr.kind, ExprKind::Closure(..)) {
            return;
        }
        walk_expr(self, expr);
    }
}

/// Returns `true` if `receiver` has type `soroban_sdk::Env`.
///
/// Its ADT definition path must match `soroban_sdk::Env`. References are peeled
/// first so `&Env` matches too.
fn is_env_receiver<'tcx>(cx: &LateContext<'tcx>, receiver: &'tcx hir::Expr<'tcx>) -> bool {
    ty_adt_def(cx.typeck_results().expr_ty(receiver).peel_refs())
        .is_some_and(|adt_def| match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "Env"]))
}

/// Returns `true` if `receiver` has type `soroban_sdk::Address`.
///
/// Its ADT definition path must match `soroban_sdk::Address`. References are
/// peeled first so `&Address` matches too.
fn is_address_receiver<'tcx>(cx: &LateContext<'tcx>, receiver: &'tcx hir::Expr<'tcx>) -> bool {
    ty_adt_def(cx.typeck_results().expr_ty(receiver).peel_refs()).is_some_and(|adt_def| {
        match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "Address"])
    })
}
