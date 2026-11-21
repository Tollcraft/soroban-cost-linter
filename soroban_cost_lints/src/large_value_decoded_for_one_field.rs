//! `large_value_decoded_for_one_field` — flags struct storage deserialization
//! where only a single field is accessed in the enclosing scope.
//!
//! Deserializing an entire multi-field struct from storage when only one field
//! is subsequently read wastes host CPU cycles and memory allocations decoding
//! unused fields.

use std::collections::{HashMap, HashSet};

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir as hir;
use rustc_hir::def::Res;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{Body, Expr, ExprKind, HirId, PatKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;
use rustc_span::Span;
use rustc_span::symbol::Symbol;

use crate::{LARGE_VALUE_DECODED_FOR_ONE_FIELD, match_soroban_def_path};

declare_lint_pass!(LargeValueDecodedForOneField => [LARGE_VALUE_DECODED_FOR_ONE_FIELD]);

const STORAGE_ACCESSOR_TIERS: [(&[&str], &str); 3] = [
    (&["soroban_sdk", "storage", "Instance"], "instance"),
    (&["soroban_sdk", "storage", "Persistent"], "persistent"),
    (&["soroban_sdk", "storage", "Temporary"], "temporary"),
];

const LINT_MSG: &str = "decoding a stored struct to read only a single field";
const LINT_HELP: &str =
    "consider storing frequently read fields under separate granular storage keys";

#[derive(Debug)]
struct TrackedBinding {
    span: Span,
    fields_accessed: HashSet<Symbol>,
    used_otherwise: bool,
}

impl<'tcx> LateLintPass<'tcx> for LargeValueDecodedForOneField {
    fn check_body(&mut self, cx: &LateContext<'tcx>, body: &Body<'tcx>) {
        let mut visitor = StructDecodeVisitor {
            cx,
            tracked: HashMap::new(),
        };
        visitor.visit_body(body);
        visitor.report_single_field_accesses();
    }
}

struct StructDecodeVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    tracked: HashMap<HirId, TrackedBinding>,
}

impl<'a, 'tcx> StructDecodeVisitor<'a, 'tcx> {
    fn report_single_field_accesses(&self) {
        for tracked in self.tracked.values() {
            if !tracked.used_otherwise && tracked.fields_accessed.len() == 1 {
                span_lint_and_help(
                    self.cx,
                    LARGE_VALUE_DECODED_FOR_ONE_FIELD,
                    tracked.span,
                    LINT_MSG,
                    None,
                    LINT_HELP,
                );
            }
        }
    }
}

impl<'tcx> Visitor<'tcx> for StructDecodeVisitor<'_, 'tcx> {
    fn visit_stmt(&mut self, stmt: &'tcx hir::Stmt<'tcx>) {
        if let hir::StmtKind::Let(local) = stmt.kind
            && let Some(init) = local.init
            && let Some(read_span) = extract_storage_read(self.cx, init)
        {
            // Track bindings in let statement
            record_let_bindings(self.cx, local.pat, read_span, &mut self.tracked);
        }
        intravisit::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Direct field access on storage get: `storage.get(...).unwrap().field`
        if let ExprKind::Field(receiver, _) = expr.kind
            && let Some(read_span) = extract_storage_read(self.cx, receiver)
        {
            let recv_ty = self.cx.typeck_results().expr_ty(receiver);
            if is_multi_field_composite(recv_ty) {
                span_lint_and_help(
                    self.cx,
                    LARGE_VALUE_DECODED_FOR_ONE_FIELD,
                    read_span,
                    LINT_MSG,
                    None,
                    LINT_HELP,
                );
                return;
            }
        }

        // Check if `expr` is a field access on a tracked variable
        if let ExprKind::Field(sub_expr, field_ident) = expr.kind {
            let peeled = peel_addr_and_deref(sub_expr);
            if let Some(local_id) = res_local_hir_id(peeled)
                && let Some(tracked) = self.tracked.get_mut(&local_id)
            {
                tracked.fields_accessed.insert(field_ident.name);
                // Do not visit sub_expr so it is not treated as an "other" bare path usage
                return;
            }
        }

        // If a tracked variable is referenced in any other way, mark used_otherwise
        if let Some(local_id) = res_local_hir_id(peel_addr_and_deref(expr))
            && let Some(tracked) = self.tracked.get_mut(&local_id)
        {
            tracked.used_otherwise = true;
        }

        intravisit::walk_expr(self, expr);
    }
}

fn record_let_bindings<'tcx>(
    cx: &LateContext<'tcx>,
    pat: &'tcx hir::Pat<'tcx>,
    span: Span,
    tracked: &mut HashMap<HirId, TrackedBinding>,
) {
    match pat.kind {
        PatKind::Binding(_, hir_id, _, _) => {
            let ty = cx.typeck_results().node_type(hir_id);
            if is_multi_field_composite(ty) {
                tracked.insert(
                    hir_id,
                    TrackedBinding {
                        span,
                        fields_accessed: HashSet::new(),
                        used_otherwise: false,
                    },
                );
            }
        }
        PatKind::Tuple(pats, _) | PatKind::Slice(pats, _, _) => {
            for p in pats {
                record_let_bindings(cx, p, span, tracked);
            }
        }
        PatKind::TupleStruct(_, pats, _) => {
            for p in pats {
                record_let_bindings(cx, p, span, tracked);
            }
        }
        _ => {}
    }
}

fn extract_storage_read<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<Span> {
    let mut cur = expr;
    while let ExprKind::MethodCall(seg, receiver, _, _) = cur.kind {
        let name = seg.ident.as_str();
        if name == "unwrap" || name == "expect" || name == "ok" || name == "unwrap_or_default" {
            cur = receiver;
        } else {
            break;
        }
    }

    if let ExprKind::MethodCall(seg, receiver, args, _span) = cur.kind {
        let name = seg.ident.as_str();
        if (name == "get" || name == "try_get") && !args.is_empty() && is_storage_tier(cx, receiver)
        {
            return Some(cur.span);
        }
    }
    None
}

fn is_storage_tier<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> bool {
    let peeled = cx.typeck_results().expr_ty(expr).peel_refs();
    let ty::Adt(adt_def, _) = peeled.kind() else {
        return false;
    };
    let def_id = adt_def.did();
    STORAGE_ACCESSOR_TIERS
        .iter()
        .any(|(segments, _)| match_soroban_def_path(cx, def_id, segments))
}

fn is_multi_field_composite(ty: ty::Ty<'_>) -> bool {
    let peeled = ty.peel_refs();
    match peeled.kind() {
        ty::Adt(adt_def, _) if adt_def.is_struct() => adt_def.all_fields().count() > 1,
        ty::Tuple(types) => types.len() > 1,
        _ => false,
    }
}

fn peel_addr_and_deref<'tcx>(mut expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    loop {
        match expr.kind {
            ExprKind::AddrOf(_, _, inner) | ExprKind::Unary(hir::UnOp::Deref, inner) => {
                expr = inner;
            }
            _ => break expr,
        }
    }
}

fn res_local_hir_id(expr: &Expr<'_>) -> Option<HirId> {
    if let ExprKind::Path(QPath::Resolved(_, path)) = expr.kind
        && let Res::Local(id) = path.res
    {
        Some(id)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_are_pinned() {
        assert_eq!(
            LINT_MSG,
            "decoding a stored struct to read only a single field"
        );
        assert_eq!(
            LINT_HELP,
            "consider storing frequently read fields under separate granular storage keys"
        );
    }

    #[test]
    fn tier_table_names_the_three_storage_accessors() {
        let tiers: Vec<&str> = STORAGE_ACCESSOR_TIERS
            .iter()
            .map(|(_, tier)| *tier)
            .collect();
        assert_eq!(tiers, ["instance", "persistent", "temporary"]);
        for (segments, _) in STORAGE_ACCESSOR_TIERS {
            assert_eq!(segments.first(), Some(&"soroban_sdk"));
        }
    }
}
