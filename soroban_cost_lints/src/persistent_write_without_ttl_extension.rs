//! `persistent_write_without_ttl_extension` — flags persistent storage writes
//! whose key does not receive an `extend_ttl` call within the same function.
//!
//! Persistent storage entries have a finite time-to-live (TTL). Writing an entry
//! without extending its TTL leaves it vulnerable to premature archival, which
//! forces an expensive, metered restoration transaction on subsequent access.

use std::collections::HashSet;

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::snippet_opt;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{Body, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;
use rustc_span::Span;
use rustc_span::def_id::DefId;

use crate::{PERSISTENT_WRITE_WITHOUT_TTL_EXTENSION, match_soroban_def_path};

declare_lint_pass!(PersistentWriteWithoutTtlExtension => [PERSISTENT_WRITE_WITHOUT_TTL_EXTENSION]);

const LINT_MSG: &str = "persistent storage write without TTL extension — archival cost cliff";
const LINT_HELP: &str = "after writing to persistent storage, call extend_ttl with an explicit threshold and extension length to avoid archival";

struct PersistentWriteVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    writes: Vec<(String, Span)>,
    extended_keys: HashSet<String>,
}

impl<'tcx> LateLintPass<'tcx> for PersistentWriteWithoutTtlExtension {
    fn check_body(&mut self, cx: &LateContext<'tcx>, body: &Body<'tcx>) {
        let mut visitor = PersistentWriteVisitor {
            cx,
            writes: Vec::new(),
            extended_keys: HashSet::new(),
        };
        visitor.visit_body(body);

        for (key, span) in visitor.writes {
            if !visitor.extended_keys.contains(&key) {
                span_lint_and_help(
                    cx,
                    PERSISTENT_WRITE_WITHOUT_TTL_EXTENSION,
                    span,
                    LINT_MSG,
                    None,
                    LINT_HELP,
                );
            }
        }
    }
}

impl<'tcx> Visitor<'tcx> for PersistentWriteVisitor<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::MethodCall(path_segment, receiver, args, _span) = expr.kind {
            let method_name = path_segment.ident.name.as_str();
            if is_persistent_storage(self.cx, receiver) {
                match method_name {
                    "set" if args.len() >= 2 => {
                        if let Some(key_text) = extract_key_snippet(self.cx, &args[0]) {
                            self.writes.push((key_text, expr.span));
                        }
                    }
                    "extend_ttl" if !args.is_empty() => {
                        if let Some(key_text) = extract_key_snippet(self.cx, &args[0]) {
                            self.extended_keys.insert(key_text);
                        }
                    }
                    _ => {}
                }
            }
        }
        intravisit::walk_expr(self, expr);
    }
}

fn is_persistent_storage<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> bool {
    let peeled = cx.typeck_results().expr_ty(expr).peel_refs();
    let ty::Adt(adt_def, _) = peeled.kind() else {
        return false;
    };
    if !match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "storage", "Persistent"]) {
        return false;
    }
    persistent_type_supports_extend_ttl(cx, adt_def.did())
}

fn persistent_type_supports_extend_ttl(cx: &LateContext<'_>, def_id: DefId) -> bool {
    if def_id.krate != rustc_hir::def_id::LOCAL_CRATE {
        return true;
    }
    for &impl_did in cx.tcx.inherent_impls(def_id) {
        for &item_id in cx.tcx.associated_item_def_ids(impl_did) {
            if cx.tcx.associated_item(item_id).name().as_str() == "extend_ttl" {
                return true;
            }
        }
    }
    false
}

fn extract_key_snippet(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<String> {
    let inner = if let ExprKind::AddrOf(_, _, e) = expr.kind {
        e
    } else {
        expr
    };
    snippet_opt(cx, inner.span).map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_are_pinned() {
        assert_eq!(
            LINT_MSG,
            "persistent storage write without TTL extension — archival cost cliff"
        );
        assert_eq!(
            LINT_HELP,
            "after writing to persistent storage, call extend_ttl with an explicit threshold and extension length to avoid archival"
        );
    }
}
