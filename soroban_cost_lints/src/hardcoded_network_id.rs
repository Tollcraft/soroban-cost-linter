use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{HARDCODED_NETWORK_ID, match_soroban_def_path, ty_adt_def};

declare_lint_pass!(HardcodedNetworkId => [HARDCODED_NETWORK_ID]);

const LEDGER_PATH: &[&str] = &["soroban_sdk", "ledger", "Ledger"];
const ENV_PATH: &[&str] = &["soroban_sdk", "Env"];

impl<'tcx> LateLintPass<'tcx> for HardcodedNetworkId {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::Binary(op, left, right) = expr.kind
            && matches!(op.node, hir::BinOpKind::Eq | hir::BinOpKind::Ne)
        {
            if is_network_id_call(cx, left) && is_hardcoded_literal(cx, right) {
                emit_lint(cx, right.span);
            } else if is_network_id_call(cx, right) && is_hardcoded_literal(cx, left) {
                emit_lint(cx, left.span);
            }
        } else if is_passphrase_literal(expr) {
            emit_lint(cx, expr.span);
        }
    }
}

fn emit_lint(cx: &LateContext<'_>, span: rustc_span::Span) {
    span_lint_and_help(
        cx,
        HARDCODED_NETWORK_ID,
        span,
        "hardcoded testnet network id in contract code",
        None,
        "retrieve the network id dynamically via `env.ledger().network_id()` instead of hardcoding a network-specific value",
    );
}

fn peel_expr<'tcx>(mut expr: &'tcx hir::Expr<'tcx>) -> &'tcx hir::Expr<'tcx> {
    loop {
        match expr.kind {
            hir::ExprKind::AddrOf(_, _, inner) => expr = inner,
            hir::ExprKind::Unary(hir::UnOp::Deref, inner) => expr = inner,
            _ => break,
        }
    }
    expr
}

fn is_network_id_call<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) -> bool {
    let unpeeled = peel_expr(expr);
    if let hir::ExprKind::MethodCall(path, receiver, _, _) = unpeeled.kind
        && path.ident.as_str() == "network_id"
    {
        let ty = cx.typeck_results().expr_ty(receiver).peel_refs();
        if let Some(adt_def) = ty_adt_def(ty) {
            return match_soroban_def_path(cx, adt_def.did(), LEDGER_PATH)
                || match_soroban_def_path(cx, adt_def.did(), ENV_PATH);
        }
    }
    false
}

fn is_hardcoded_literal<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) -> bool {
    let unpeeled = peel_expr(expr);
    let ty = cx.typeck_results().expr_ty(unpeeled).peel_refs();
    if let rustc_middle::ty::Array(elem_ty, len) = ty.kind()
        && elem_ty.is_integral()
        && let Some(32) = len.try_to_target_usize(cx.tcx)
    {
        return matches!(
            unpeeled.kind,
            hir::ExprKind::Array(_) | hir::ExprKind::Repeat(..) | hir::ExprKind::Lit(_)
        ) || is_const_path(unpeeled);
    }
    false
}

fn is_const_path(expr: &hir::Expr<'_>) -> bool {
    if let hir::ExprKind::Path(hir::QPath::Resolved(None, path)) = expr.kind {
        matches!(
            path.res,
            hir::def::Res::Def(rustc_hir::def::DefKind::Const { .. }, _)
                | hir::def::Res::Def(rustc_hir::def::DefKind::AssocConst { .. }, _)
                | hir::def::Res::Def(rustc_hir::def::DefKind::Static { .. }, _)
        )
    } else {
        false
    }
}

fn is_passphrase_literal(expr: &hir::Expr<'_>) -> bool {
    let unpeeled = peel_expr(expr);
    if let hir::ExprKind::Lit(lit) = &unpeeled.kind
        && let rustc_ast::LitKind::Str(sym, _) = &lit.node
    {
        let s = sym.as_str();
        return s.contains("Test SDF Network")
            || s.contains("Public Global Stellar Network")
            || s.contains("Test SDF Future Network");
    }
    false
}
