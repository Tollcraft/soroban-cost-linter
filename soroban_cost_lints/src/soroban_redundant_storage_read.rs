//! `soroban_redundant_storage_read` — flags a storage read of a key that was
//! already read, unchanged, earlier in the same block.
//!
//! Every storage read is metered, so reading the same key twice without writing
//! in between pays twice for the same value. Storing the first result and
//! reusing it is free.
//!
//! Detection is per block and only looks at top-level statements: the tracker
//! remembers the last read as `(storage accessor, key)`, clears it on a `set`,
//! and reports a read that matches what is already remembered. Expressions
//! nested deeper than the block's statement list are not tracked, which keeps
//! the analysis linear in the number of statements without a full dataflow
//! pass.

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::snippet_opt;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;
use rustc_span::def_id::DefId;

use crate::{SOROBAN_REDUNDANT_STORAGE_READ, cached_def_path_str};

declare_lint_pass!(SorobanRedundantStorageRead => [SOROBAN_REDUNDANT_STORAGE_READ]);

/// Storage accessor types, as `(exact, anchored)` path-suffix pairs.
///
/// Both forms are needed to match the accepted spellings: a full build has the
/// SDK path prefixed by a crate root, while a bare path equals the suffix. See
/// [`is_storage_accessor_path`].
const STORAGE_ACCESSOR_SUFFIXES: &[(&str, &str)] = &[
    (
        "soroban_sdk::storage::Instance",
        "::soroban_sdk::storage::Instance",
    ),
    (
        "soroban_sdk::storage::Persistent",
        "::soroban_sdk::storage::Persistent",
    ),
    (
        "soroban_sdk::storage::Temporary",
        "::soroban_sdk::storage::Temporary",
    ),
];

/// Observing methods this lint tracks. `has` counts as a read: it inspects the
/// key without changing it, so a following `get` still pays for the same value.
const STORAGE_READ_METHODS: &[&str] = &["get", "has"];

/// The write method that clears a tracked read.
const STORAGE_WRITE_METHOD: &str = "set";

/// Message reported for a repeat read of an unchanged key.
const SOROBAN_REDUNDANT_STORAGE_READ_MSG: &str =
    "redundant storage read: this key was already read without modification";

/// Help note attached to [`SOROBAN_REDUNDANT_STORAGE_READ_MSG`].
const SOROBAN_REDUNDANT_STORAGE_READ_HELP: &str =
    "store the value from the first read and reuse it instead of reading again";

/// Whether `full` names one of the storage accessor types.
///
/// This mirrors the shared [`crate::match_soroban_def_path`] rule for these
/// three paths, but checks precomputed suffix pairs against the cached path
/// string. That avoids the `join` and `format!` allocations the generic helper
/// performs on every call, which matters here because this runs once per
/// storage method call the lint visits.
fn is_storage_accessor_path(full: &str) -> bool {
    STORAGE_ACCESSOR_SUFFIXES.iter().any(|(exact, anchored)| {
        full == *exact || (full.ends_with(*anchored) && is_soroban_root(full))
    })
}

/// Whether the first segment of a definition path is one of the crate names the
/// SDK is known to be compiled under.
fn is_soroban_root(full: &str) -> bool {
    matches!(
        full.split("::").next().unwrap_or(""),
        "soroban_sdk" | "soroban_env_host" | "soroban_env_common"
    )
}

/// The `DefId` behind `ty` when it is one of the storage accessor types, after
/// peeling references.
///
/// The definition path is resolved once and read from the per-`DefId` cache,
/// rather than being formatted three times (once per accessor type).
fn storage_accessor_def_id<'tcx>(cx: &LateContext<'tcx>, ty: ty::Ty<'tcx>) -> Option<DefId> {
    let peeled = ty.peel_refs();
    if let ty::Adt(adt_def, _) = peeled.kind() {
        let did = adt_def.did();
        if is_storage_accessor_path(&cached_def_path_str(cx.tcx, did)) {
            return Some(did);
        }
    }
    None
}

/// A storage access found in a block, classified by whether it observes or
/// overwrites the key.
enum StorageOp {
    /// A `get`/`has` on `storage_def_id` whose key prints as `key_text`.
    Read {
        storage_def_id: DefId,
        key_text: String,
    },
    /// A `set`, which invalidates any tracked read.
    Write,
}

/// Classifies a top-level expression as a storage read or write, or `None` when
/// it is neither.
fn extract_storage_op<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx hir::Expr<'tcx>,
) -> Option<StorageOp> {
    let hir::ExprKind::MethodCall(path_segment, receiver, args, _span) = expr.kind else {
        return None;
    };

    let method_name = path_segment.ident.name.as_str();
    if !STORAGE_READ_METHODS.contains(&method_name) && method_name != STORAGE_WRITE_METHOD {
        return None;
    }

    let storage_def_id = storage_accessor_def_id(cx, cx.typeck_results().expr_ty(receiver))?;

    if method_name == STORAGE_WRITE_METHOD {
        return Some(StorageOp::Write);
    }

    // `get`/`has` take the key by reference; strip the `&` so the text of the
    // key itself is compared rather than the borrow expression.
    let key_arg = args.first()?;
    let key_inner = if let hir::ExprKind::AddrOf(_, _, inner) = key_arg.kind {
        inner
    } else {
        key_arg
    };

    Some(StorageOp::Read {
        storage_def_id,
        key_text: snippet_opt(cx, key_inner.span)?,
    })
}

/// Late pass backing [`SOROBAN_REDUNDANT_STORAGE_READ`].
impl<'tcx> LateLintPass<'tcx> for SorobanRedundantStorageRead {
    /// Walks a block's top-level statements, remembering the most recent read
    /// and reporting a read that repeats it without an intervening `set`.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx hir::Block<'tcx>) {
        let mut last_read: Option<(DefId, String)> = None;

        let exprs = block
            .stmts
            .iter()
            .filter_map(|stmt| match stmt.kind {
                hir::StmtKind::Let(&hir::LetStmt {
                    init: Some(init), ..
                }) => Some(init),
                hir::StmtKind::Expr(expr) | hir::StmtKind::Semi(expr) => Some(expr),
                _ => None,
            })
            .chain(block.expr);

        for expr in exprs {
            if let Some(op) = extract_storage_op(cx, expr) {
                match op {
                    StorageOp::Read {
                        storage_def_id,
                        key_text,
                    } => {
                        if let Some((last_def_id, ref last_key)) = last_read
                            && last_def_id == storage_def_id
                            && *last_key == key_text
                        {
                            span_lint_and_help(
                                cx,
                                SOROBAN_REDUNDANT_STORAGE_READ,
                                expr.span,
                                SOROBAN_REDUNDANT_STORAGE_READ_MSG,
                                None,
                                SOROBAN_REDUNDANT_STORAGE_READ_HELP,
                            );
                        }
                        last_read = Some((storage_def_id, key_text));
                    }
                    StorageOp::Write => {
                        last_read = None;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SOROBAN_REDUNDANT_STORAGE_READ_HELP, SOROBAN_REDUNDANT_STORAGE_READ_MSG,
        STORAGE_READ_METHODS, STORAGE_WRITE_METHOD, is_soroban_root, is_storage_accessor_path,
    };

    #[test]
    fn accessor_paths_match_the_three_storage_tiers() {
        for path in [
            "soroban_sdk::storage::Instance",
            "soroban_sdk::storage::Persistent",
            "soroban_sdk::storage::Temporary",
        ] {
            assert!(is_storage_accessor_path(path));
        }
        assert!(!is_storage_accessor_path("soroban_sdk::storage::Storage"));
        assert!(!is_storage_accessor_path("core::option::Option"));
    }

    #[test]
    fn anchored_paths_require_a_soroban_root() {
        assert!(is_storage_accessor_path(
            "soroban_env_host::soroban_sdk::storage::Instance"
        ));
        assert!(!is_storage_accessor_path("other_crate::storage::Instance"));
    }

    #[test]
    fn roots_are_limited_to_known_sdk_crates() {
        assert!(is_soroban_root("soroban_sdk::storage::Instance"));
        assert!(is_soroban_root("soroban_env_host::storage::Instance"));
        assert!(is_soroban_root("soroban_env_common::storage::Instance"));
        assert!(!is_soroban_root("my_crate::storage::Instance"));
    }

    #[test]
    fn method_tables_and_messages_are_pinned() {
        assert_eq!(STORAGE_READ_METHODS, ["get", "has"]);
        assert_eq!(STORAGE_WRITE_METHOD, "set");
        assert!(!SOROBAN_REDUNDANT_STORAGE_READ_MSG.is_empty());
        assert!(!SOROBAN_REDUNDANT_STORAGE_READ_HELP.is_empty());
    }
}
