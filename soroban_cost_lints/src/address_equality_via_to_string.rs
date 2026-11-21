use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg};
use clippy_utils::source::snippet_opt;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{ADDRESS_EQUALITY_VIA_TO_STRING, is_type_match};

declare_lint_pass!(AddressEqualityViaToString => [ADDRESS_EQUALITY_VIA_TO_STRING]);

const SOROBAN_ADDRESS_TYPE: &[&[&str]] = &[&["soroban_sdk", "Address"]];

impl<'tcx> LateLintPass<'tcx> for AddressEqualityViaToString {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        let hir::ExprKind::Binary(op, lhs, rhs) = expr.kind else {
            return;
        };
        let operator = match op.node {
            hir::BinOpKind::Eq => "==",
            hir::BinOpKind::Ne => "!=",
            _ => return,
        };
        let (Some(lhs), Some(rhs)) = (
            address_to_string_receiver(cx, lhs),
            address_to_string_receiver(cx, rhs),
        ) else {
            return;
        };

        if let (Some(lhs), Some(rhs)) = (snippet_opt(cx, lhs.span), snippet_opt(cx, rhs.span)) {
            span_lint_and_sugg(
                cx,
                ADDRESS_EQUALITY_VIA_TO_STRING,
                expr.span,
                "comparing Address values through their string representations",
                "compare the Address values directly",
                format!("{} {operator} {}", lhs.trim(), rhs.trim()),
                Applicability::MachineApplicable,
            );
        } else {
            span_lint_and_help(
                cx,
                ADDRESS_EQUALITY_VIA_TO_STRING,
                expr.span,
                "comparing Address values through their string representations",
                None,
                "compare the Address values directly",
            );
        }
    }
}

fn address_to_string_receiver<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx hir::Expr<'tcx>,
) -> Option<&'tcx hir::Expr<'tcx>> {
    let hir::ExprKind::MethodCall(path_segment, receiver, _args, _) = expr.kind else {
        return None;
    };
    if path_segment.ident.name.as_str() != "to_string"
        || !is_type_match(
            cx,
            cx.typeck_results().expr_ty(receiver),
            SOROBAN_ADDRESS_TYPE,
        )
    {
        return None;
    }
    Some(receiver)
}
