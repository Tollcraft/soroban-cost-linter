//! `map_iter_then_lookup` — flags loops iterating over `map.keys()` where the
//! loop body looks up each key on the same map instance.
//!
//! Iterating over `map.keys()` and calling `map.get(&k)` inside the loop pays
//! quadratic host traversal cost where `map.iter()` provides direct access to
//! both key and value.

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::higher;
use clippy_utils::source::snippet_opt;
use rustc_hir as hir;
use rustc_hir::intravisit::{self, Visitor};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{MAP_ITER_THEN_LOOKUP, match_soroban_def_path, ty_adt_def};

declare_lint_pass!(MapIterThenLookup => [MAP_ITER_THEN_LOOKUP]);

const MAP_ITER_THEN_LOOKUP_MSG: &str = "iterating map keys and looking up each key is quadratic";

const MAP_ITER_THEN_LOOKUP_HELP: &str = "use `map.iter()` to iterate over key-value pairs directly";

impl<'tcx> LateLintPass<'tcx> for MapIterThenLookup {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        let Some(higher::ForLoop { pat, arg, body, .. }) = higher::ForLoop::hir(expr) else {
            return;
        };

        let hir::ExprKind::MethodCall(segment, map_receiver, _args, _) = arg.kind else {
            return;
        };

        if segment.ident.name.as_str() != "keys" {
            return;
        }

        let receiver_ty = cx.typeck_results().expr_ty(map_receiver);
        if !is_soroban_map(cx, receiver_ty) {
            return;
        }

        let mut loop_var_ids = Vec::new();
        pat.walk_short(|p| {
            if let hir::PatKind::Binding(_, hir_id, _, _) = p.kind {
                loop_var_ids.push(hir_id);
            }
            true
        });

        if loop_var_ids.is_empty() {
            return;
        }

        let mut visitor = KeyLookupVisitor {
            cx,
            map_receiver,
            loop_var_ids: &loop_var_ids,
            found: false,
        };
        visitor.visit_expr(body);

        if visitor.found {
            span_lint_and_help(
                cx,
                MAP_ITER_THEN_LOOKUP,
                expr.span,
                MAP_ITER_THEN_LOOKUP_MSG,
                None,
                MAP_ITER_THEN_LOOKUP_HELP,
            );
        }
    }
}

fn is_soroban_map(cx: &LateContext<'_>, ty: rustc_middle::ty::Ty<'_>) -> bool {
    let peeled = ty.peel_refs();
    if let Some(adt) = ty_adt_def(peeled) {
        match_soroban_def_path(cx, adt.did(), &["soroban_sdk", "Map"])
            || match_soroban_def_path(cx, adt.did(), &["soroban_sdk", "map", "Map"])
    } else {
        false
    }
}

struct KeyLookupVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    map_receiver: &'tcx hir::Expr<'tcx>,
    loop_var_ids: &'a [hir::HirId],
    found: bool,
}

impl<'tcx> Visitor<'tcx> for KeyLookupVisitor<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        if self.found {
            return;
        }

        if let hir::ExprKind::MethodCall(segment, receiver, args, _) = expr.kind {
            let method_name = segment.ident.name.as_str();
            if (method_name == "get" || method_name == "get_unchecked")
                && args.len() == 1
                && is_same_receiver(self.cx, self.map_receiver, receiver)
                && expr_uses_binding(&args[0], self.loop_var_ids)
            {
                self.found = true;
                return;
            }
        }

        intravisit::walk_expr(self, expr);
    }
}

fn is_same_receiver(cx: &LateContext<'_>, a: &hir::Expr<'_>, b: &hir::Expr<'_>) -> bool {
    let mut cur_a = a;
    while let hir::ExprKind::AddrOf(_, _, inner) = cur_a.kind {
        cur_a = inner;
    }
    let mut cur_b = b;
    while let hir::ExprKind::AddrOf(_, _, inner) = cur_b.kind {
        cur_b = inner;
    }

    if let (
        hir::ExprKind::Path(hir::QPath::Resolved(None, path_a)),
        hir::ExprKind::Path(hir::QPath::Resolved(None, path_b)),
    ) = (&cur_a.kind, &cur_b.kind)
        && let (hir::def::Res::Local(id_a), hir::def::Res::Local(id_b)) = (path_a.res, path_b.res)
    {
        return id_a == id_b;
    }

    if let (Some(snip_a), Some(snip_b)) = (snippet_opt(cx, cur_a.span), snippet_opt(cx, cur_b.span))
    {
        return snip_a.trim() == snip_b.trim();
    }

    false
}

fn expr_uses_binding(expr: &hir::Expr<'_>, loop_var_ids: &[hir::HirId]) -> bool {
    let mut current = expr;
    while let hir::ExprKind::AddrOf(_, _, inner) = current.kind {
        current = inner;
    }
    while let hir::ExprKind::Unary(hir::UnOp::Deref, inner) = current.kind {
        current = inner;
    }

    if let hir::ExprKind::Path(hir::QPath::Resolved(None, path)) = current.kind
        && let hir::def::Res::Local(id) = path.res
    {
        return loop_var_ids.contains(&id);
    }

    false
}
