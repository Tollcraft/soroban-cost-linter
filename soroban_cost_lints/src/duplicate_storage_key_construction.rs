use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::LitKind;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use crate::DUPLICATE_STORAGE_KEY_CONSTRUCTION;

rustc_session::impl_lint_pass!(DuplicateStorageKeyConstruction => [DUPLICATE_STORAGE_KEY_CONSTRUCTION]);

/// One construction site: the enclosing function's owner and the call span.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Site {
    owner: LocalDefId,
    span: Span,
}

/**
 * Emits `duplicate_storage_key_construction` when the same storage key is
 * constructed in more than one function body.
 *
 * The lint marks every construction whose key literal also appears in a
 * different function — the first definition is effectively the shared one,
 * and every other function rebuilding the same key is a copy that can drift
 * into a silent, undebuggable state bug while paying the symbol-construction
 * host call again (see `LintMeta`). Keys referenced through a named constant,
 * or built from a runtime-derived value, are not tracked: a constant is the
 * idiomatic single-construction fix, and a runtime payload cannot be
 * identified statically.
 *
 * Sites are buffered per key literal; a site's diagnostic flushes as soon as
 * a second function shows the same key, so each participating function sees
 * exactly one diagnostic for the shared key once duplication is established.
 */
#[derive(Default)]
pub struct DuplicateStorageKeyConstruction {
    /// Key literal → construction sites, in first-visit order.
    sites: RefCell<HashMap<String, Vec<Site>>>,
    /// Sites already diagnosed, keyed by span.
    warned: RefCell<HashSet<Span>>,
}

impl DuplicateStorageKeyConstruction {
    fn record_site(&self, cx: &LateContext<'_>, key: &str, site: Site) {
        let mut sites = self.sites.borrow_mut();
        let entries = sites.entry(key.to_string()).or_default();

        // The same key constructed twice in one function body is a different
        // concern (an element of the same body): the description is about
        // multiple function bodies.
        if entries.iter().any(|s| s.owner == site.owner) {
            return;
        }
        entries.push(site);

        let distinct_owners: HashSet<LocalDefId> = entries.iter().map(|s| s.owner).collect();
        if distinct_owners.len() < 2 {
            return;
        }

        // Duplication is established: diagnose every site not yet emitted,
        // including earlier sites buffered before the second body showed up.
        let mut warned = self.warned.borrow_mut();
        for s in entries.iter().copied() {
            if warned.insert(s.span) {
                span_lint_and_help(
                    cx,
                    DUPLICATE_STORAGE_KEY_CONSTRUCTION,
                    s.span,
                    format!("storage key `{key}` is constructed in more than one function body"),
                    None,
                    "construct the key once — as a named constant or in a single accessor — so \
                     every storage access shares one construction instead of paying the \
                     symbol-construction host call again",
                );
            }
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for DuplicateStorageKeyConstruction {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::Call(func, args) = expr.kind else {
            return;
        };
        // Recognise `Symbol::new(env, <literal>)`. An associated-function call
        // on a type reaches HIR as `QPath::TypeRelative(Ty, "new")` with the
        // type's own path as the receiver type (`Symbol::new` with `Symbol`
        // imported directly, `soroban_sdk::Symbol::new` fully qualified) — the
        // bare `QPath::Resolved` two-segment shape is the trait/generic form,
        // which would also match unrelated `into_iter` calls. Match the type's
        // final segment and the call's final segment.
        let (type_last, call_seg) = match func.kind {
            ExprKind::Path(QPath::TypeRelative(ty, seg)) => match ty.kind {
                rustc_hir::TyKind::Path(QPath::Resolved(_, ty_path)) => {
                    (ty_path.segments.last(), Some(seg.ident.as_str()))
                }
                _ => (None, None),
            },
            ExprKind::Path(QPath::Resolved(_, p)) if p.segments.len() >= 2 => (
                p.segments.get(p.segments.len() - 2),
                Some(p.segments.last().unwrap().ident.as_str()),
            ),
            _ => (None, None),
        };
        let is_symbol_new =
            type_last.is_some_and(|seg| seg.ident.as_str() == "Symbol") && call_seg == Some("new");
        if !is_symbol_new {
            return;
        }

        // Only a compile-time-known string payload is a tracked key. A path
        // argument (a named constant) and any non-literal expression are the
        // accepted, single-construction shapes.
        let ExprKind::Lit(lit) = args[1].kind else {
            return;
        };
        let LitKind::Str(symbol, _) = lit.node else {
            return;
        };

        let owner = cx.tcx.hir_enclosing_body_owner(expr.hir_id);
        self.record_site(
            cx,
            symbol.as_str(),
            Site {
                owner,
                span: expr.span,
            },
        );
    }
}
