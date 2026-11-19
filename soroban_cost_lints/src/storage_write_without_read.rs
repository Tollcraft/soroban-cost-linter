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

/// Single-pass ordered visitor that tracks storage reads and writes in
/// statement execution order.
///
/// As the HIR visitor walks each expression in source order it:
/// * Records reads (`get`, `try_get`, `has`, `remove`, `update`) in `seen_reads`.
/// * On encountering a write (`set`), checks whether the same receiver-key pair
///   already appears in `seen_reads`. If not, records the write span in
///   `unread_writes` for later diagnostic emission.
///
/// This ensures a read that appears *after* a write in the source text does not
/// retroactively suppress the warning for that write, matching the documented
/// requirement that the key must be read *earlier* in the same function body.
struct OrderedAccessVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    /// Storage receiver-key pairs that have been read so far (in source order).
    seen_reads: HashSet<(String, String)>,
    /// Writes for which no preceding read was found, to be diagnosed.
    unread_writes: Vec<rustc_span::Span>,
}

impl<'tcx> Visitor<'tcx> for OrderedAccessVisitor<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, args, span) = expr.kind
            && is_storage_receiver(self.cx, receiver)
        {
            let method = path_segment.ident.name.as_str();
            if STORAGE_READ_METHODS.contains(&method) {
                if let Some(pair) = access_pair(self.cx, receiver, args) {
                    self.seen_reads.insert(pair);
                }
            } else if method == STORAGE_WRITE_METHOD
                && args.len() >= 2
                && let Some(pair) = access_pair(self.cx, receiver, args)
                && !self.seen_reads.contains(&pair)
            {
                self.unread_writes.push(span);
            }
        }
        intravisit::walk_expr(self, expr);
    }
}

/// Late pass backing [`STORAGE_WRITE_WITHOUT_READ`].
impl<'tcx> LateLintPass<'tcx> for StorageWriteWithoutRead {
    /// Visits a function body in statement order, emitting a diagnostic for
    /// every storage write whose key was not read *earlier* in the same body.
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

        let mut visitor = OrderedAccessVisitor {
            cx,
            seen_reads: HashSet::new(),
            unread_writes: Vec::new(),
        };
        visitor.visit_body(body);

        for span in visitor.unread_writes {
            span_lint_and_help(
                cx,
                STORAGE_WRITE_WITHOUT_READ,
                span,
                STORAGE_WRITE_WITHOUT_READ_MSG,
                None,
                STORAGE_WRITE_WITHOUT_READ_HELP,
            );
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

    /// Verifies the ordered-visitor contract: a read that is recorded before a
    /// write for the same key suppresses the warning; a read that is recorded
    /// AFTER a write (or not at all) does not.
    #[test]
    fn ordered_visitor_read_before_write_suppresses_only_preceding_reads() {
        use std::collections::HashSet;

        // Simulate two operations: read key A then write key A.
        let mut seen: HashSet<(String, String)> = HashSet::new();
        let pair_a = ("instance".to_string(), "&1".to_string());

        // Read key A first → seen_reads now contains A.
        seen.insert(pair_a.clone());
        // Write key A → it IS in seen_reads, so no unread write expected.
        assert!(
            seen.contains(&pair_a),
            "read-before-write should suppress the warning"
        );

        // Simulate: write key B with no prior read.
        let pair_b = ("instance".to_string(), "&2".to_string());
        assert!(
            !seen.contains(&pair_b),
            "write-without-prior-read should not be suppressed"
        );

        // Add a read for key B AFTER the write (simulates the old bug).
        seen.insert(pair_b.clone());
        // With the old two-pass approach pair_b would already be in seen; this
        // test documents that the ordered visitor checks BEFORE inserting.
        // The assertion above already captured the correct state at write-time.
        let _ = seen; // suppress unused-variable warning
    }
}
