//! `storage_write_without_read` — flags a storage write whose key was not read
//! earlier in the same function body.
//!
//! Soroban meters storage reads and writes, and a write that has no preceding
//! read overwrites the stored value without ever looking at it. That is usually
//! either a logic error (the previous value mattered) or a deliberate
//! initialisation. Deliberate initialisers are recognised by function name — see
//! [`is_initialiser`] — and left alone; everything else is reported so the
//! author can confirm the overwrite is intended.
//!
//! Detection is per function body. A write is reported only when none of the
//! observing methods in [`STORAGE_READ_METHODS`] touched the *same* receiver and
//! key earlier in the body. Receiver and key are compared by their source text,
//! so two expressions that print identically are treated as the same
//! receiver/key even if they are distinct bindings. That keeps the analysis
//! cheap and free of HIR-span plumbing at the cost of a known imprecision.

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::snippet_opt;
use rustc_hir as hir;
use rustc_hir::intravisit::{self, Visitor};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use std::collections::HashSet;

use crate::{SOROBAN_STORAGE_TYPES, STORAGE_WRITE_WITHOUT_READ, matches_any_path, ty_adt_def};

declare_lint_pass!(StorageWriteWithoutRead => [STORAGE_WRITE_WITHOUT_READ]);

/// Accessor methods that observe a key without overwriting it. A `set` that
/// follows one of these on the same receiver-key pair is intentional and is not
/// reported: `remove` and `update` are included because both read the current
/// value before acting on it.
const STORAGE_READ_METHODS: &[&str] = &["get", "try_get", "has", "remove", "update"];

/// The write method this lint pairs with a preceding read.
const STORAGE_WRITE_METHOD: &str = "set";

/// Message reported for a write whose key was not read first.
const STORAGE_WRITE_WITHOUT_READ_MSG: &str = "storage write without a corresponding read";

/// Help note attached to [`STORAGE_WRITE_WITHOUT_READ_MSG`].
const STORAGE_WRITE_WITHOUT_READ_HELP: &str =
    "consider reading the value before writing or using `.has()` to check existence";

/// Function names treated as deliberate initialisers, where writing before any
/// read is expected: constructors and admin setup. Matching is intentionally
/// substring-based so `initialize`, `init_owner`, and `set_admin_key` are all
/// covered by one rule, at the cost of also matching unrelated names that
/// happen to contain `init` or `set_admin`.
fn is_initialiser(fn_name: &str) -> bool {
    fn_name.contains("init") || fn_name.contains("set_admin")
}

/// Whether `receiver` is one of the Soroban storage accessor types
/// ([`SOROBAN_STORAGE_TYPES`]), after peeling references.
fn is_storage_receiver<'tcx>(cx: &LateContext<'tcx>, receiver: &'tcx hir::Expr<'tcx>) -> bool {
    ty_adt_def(cx.typeck_results().expr_ty(receiver).peel_refs())
        .is_some_and(|adt_def| matches_any_path(cx, adt_def.did(), SOROBAN_STORAGE_TYPES))
}

/// `(receiver snippet, key snippet)` for a storage access, or `None` when the
/// access does not have the shape the lint compares.
fn access_pair<'tcx>(
    cx: &LateContext<'tcx>,
    receiver: &'tcx hir::Expr<'tcx>,
    args: &'tcx [hir::Expr<'tcx>],
) -> Option<(String, String)> {
    let key = args.first()?;
    let receiver_snippet = snippet_opt(cx, receiver.span).unwrap_or_default();
    let key_snippet = snippet_opt(cx, key.span).unwrap_or_default();
    Some((receiver_snippet, key_snippet))
}

/// Collects storage reads (`get`, `try_get`, `has`, `remove`, `update`) as
/// `(receiver, key)` source-text pairs for later cross-referencing.
struct ReadVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    reads: HashSet<(String, String)>,
}

impl<'tcx> Visitor<'tcx> for ReadVisitor<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, args, _span) = expr.kind
            && STORAGE_READ_METHODS.contains(&path_segment.ident.name.as_str())
            && is_storage_receiver(self.cx, receiver)
            && let Some(pair) = access_pair(self.cx, receiver, args)
        {
            self.reads.insert(pair);
        }
        intravisit::walk_expr(self, expr);
    }
}

/// Collects storage writes (`set`) with receiver, key, and span for later
/// comparison against the read set.
struct WriteVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    writes: Vec<(String, String, rustc_span::Span)>,
}

impl<'tcx> Visitor<'tcx> for WriteVisitor<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, args, span) = expr.kind
            && path_segment.ident.name.as_str() == STORAGE_WRITE_METHOD
            && args.len() >= 2
            && is_storage_receiver(self.cx, receiver)
            && let Some((receiver_snippet, key_snippet)) = access_pair(self.cx, receiver, args)
        {
            self.writes.push((receiver_snippet, key_snippet, span));
        }
        intravisit::walk_expr(self, expr);
    }
}

/// Late pass backing [`STORAGE_WRITE_WITHOUT_READ`].
impl<'tcx> LateLintPass<'tcx> for StorageWriteWithoutRead {
    /// Visits a function body, collecting reads then writes, and emits a
    /// diagnostic for every write whose receiver-key pair was not read first.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: intravisit::FnKind<'tcx>,
        _: &'tcx hir::FnDecl<'tcx>,
        body: &'tcx hir::Body<'tcx>,
        _: rustc_span::Span,
        def_id: rustc_hir::def_id::LocalDefId,
    ) {
        // Initialisers are expected to write before reading anything.
        if let Some(name) = cx.tcx.opt_item_name(def_id.to_def_id())
            && is_initialiser(name.as_str())
        {
            return;
        }

        let mut read_visitor = ReadVisitor {
            cx,
            reads: HashSet::new(),
        };
        read_visitor.visit_body(body);

        let mut write_visitor = WriteVisitor {
            cx,
            writes: Vec::new(),
        };
        write_visitor.visit_body(body);

        for (receiver, key, span) in &write_visitor.writes {
            let has_read = read_visitor
                .reads
                .iter()
                .any(|(read_receiver, read_key)| read_receiver == receiver && read_key == key);
            if !has_read {
                span_lint_and_help(
                    cx,
                    STORAGE_WRITE_WITHOUT_READ,
                    *span,
                    STORAGE_WRITE_WITHOUT_READ_MSG,
                    None,
                    STORAGE_WRITE_WITHOUT_READ_HELP,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        STORAGE_READ_METHODS, STORAGE_WRITE_METHOD, STORAGE_WRITE_WITHOUT_READ_HELP,
        STORAGE_WRITE_WITHOUT_READ_MSG, is_initialiser,
    };

    #[test]
    fn read_methods_cover_every_observing_access() {
        for method in ["get", "try_get", "has", "remove", "update"] {
            assert!(
                STORAGE_READ_METHODS.contains(&method),
                "{method} should count as a read",
            );
        }
        assert!(!STORAGE_READ_METHODS.contains(&"set"));
    }

    #[test]
    fn write_method_is_set() {
        assert_eq!(STORAGE_WRITE_METHOD, "set");
    }

    #[test]
    fn initialisers_are_recognised_by_substring() {
        for name in ["initialize", "init_owner", "set_admin", "set_admin_key"] {
            assert!(is_initialiser(name), "{name} should be an initialiser");
        }
        for name in ["transfer", "record_set", "read_admin_flag"] {
            assert!(!is_initialiser(name), "{name} should not be an initialiser");
        }
    }

    #[test]
    fn message_and_help_are_non_empty() {
        assert!(!STORAGE_WRITE_WITHOUT_READ_MSG.is_empty());
        assert!(!STORAGE_WRITE_WITHOUT_READ_HELP.is_empty());
    }
}
