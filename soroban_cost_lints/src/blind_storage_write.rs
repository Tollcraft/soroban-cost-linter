//! `blind_storage_write` — flags a storage write that overwrites a value the
//! same function has already written, without reading it back in between.
//!
//! Soroban meters every storage write. A function that writes a key, then writes
//! it again before anything observes the first store, has paid twice for one
//! surviving value: the first write is silently discarded. This is the
//! stale-write hazard the lint page describes and is usually either dead work
//! or a missing read.
//!
//! # Relationship to `storage_write_without_read`
//!
//! The two lints answer different questions and both stay silent on each
//! other's territory:
//!
//! - `storage_write_without_read` reports a write whose key is **never** read
//!   anywhere in the function.
//! - `blind_storage_write` reports a write that overwrites an earlier write of
//!   the **same** key, in a function where that key **is** read at least once.
//!
//! A never-read key therefore produces only the sibling lint's diagnostics, and
//! a write informed by a fresh read produces neither — the ui fixture records
//! both halves of that split.
//!
//! # Detection
//!
//! One function body at a time, in source order. Each `(storage tier, key)` pair
//! carries two facts:
//!
//! - whether the key was ever read in this body, and
//! - whether it has been written since the last read.
//!
//! `get`/`try_get`/`has` clear the second fact and set the first; `set`/`put`/
//! `update` report when both are set (a read happened earlier, and no read has
//! happened since the previous write), then mark the key as written.
//!
//! Receivers are matched through [`SOROBAN_STORAGE_TYPES`] with
//! [`matches_any_path`] (never by type-name string), and the tier is part of the
//! key so the same key text in `instance` and `persistent` storage stays
//! distinct. The key itself is the argument's span-normalised source text —
//! whitespace stripped and integer suffixes removed — so `&1`, `& 1` and `&1i32`
//! name the same key.

use std::collections::HashMap;

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::snippet_opt;
use rustc_hir as hir;
use rustc_hir::intravisit::{self, Visitor};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{
    BLIND_STORAGE_WRITE, SOROBAN_STORAGE_TYPES, cached_def_path_str, matches_any_path, ty_adt_def,
};

declare_lint_pass!(BlindStorageWrite => [BLIND_STORAGE_WRITE]);

/// Storage methods that observe a key's value. `has` counts: a write guarded by
/// an existence check has consulted the stored state.
const STORAGE_READ_METHODS: &[&str] = &["get", "try_get", "has"];

/// Storage methods that store a value over whatever the key held before.
const STORAGE_WRITE_METHODS: &[&str] = &["set", "put", "update"];

/// Message reported for a write that discards a previous store unseen.
const BLIND_STORAGE_WRITE_MSG: &str =
    "blind storage write overwrites a previously written key without reading it back";

/// Help note attached to [`BLIND_STORAGE_WRITE_MSG`]; the wording is pinned by
/// the ui golden.
const BLIND_STORAGE_WRITE_HELP: &str = "read the previous value (or guard with `.has()`) before \
     overwriting, otherwise the earlier store is silently discarded";

/// What the function body has done to one `(tier, key)` so far.
#[derive(Default)]
struct KeyState {
    /// The key was read at least once in this body. Without this, a blind
    /// overwrite is `storage_write_without_read`'s business, not ours.
    ever_read: bool,
    /// A write happened after the most recent read.
    written_since_read: bool,
}

/// Walks one function body in source order, carrying the per-key state.
struct BlindStorageWriteVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    states: HashMap<(String, String), KeyState>,
}

impl<'tcx> Visitor<'tcx> for BlindStorageWriteVisitor<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        self.check_call(expr);
        intravisit::walk_expr(self, expr);
    }
}

impl<'tcx> BlindStorageWriteVisitor<'_, 'tcx> {
    fn check_call(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        let hir::ExprKind::MethodCall(segment, receiver, args, _) = expr.kind else {
            return;
        };
        let method = segment.ident.as_str();
        let is_read = STORAGE_READ_METHODS.contains(&method);
        let is_write = STORAGE_WRITE_METHODS.contains(&method);
        if !is_read && !is_write {
            return;
        }

        let Some(adt_def) = ty_adt_def(self.cx.typeck_results().expr_ty(receiver).peel_refs())
            .filter(|adt_def| matches_any_path(self.cx, adt_def.did(), SOROBAN_STORAGE_TYPES))
        else {
            return;
        };
        let Some(key_expr) = args.first() else {
            return;
        };
        let Some(key_text) = snippet_opt(self.cx, key_expr.span) else {
            return;
        };
        // The accessor type is the tier: `instance`, `persistent` or
        // `temporary`. Two tiers may hold the same key with independent state.
        let tier = cached_def_path_str(self.cx.tcx, adt_def.did());
        let state = self
            .states
            .entry((tier, normalize_key_text(&key_text)))
            .or_default();

        if is_read {
            state.ever_read = true;
            state.written_since_read = false;
            return;
        }

        if state.ever_read && state.written_since_read {
            // Span the call itself (`set(&k, &v)`), not the receiver chain, to
            // match the other storage lints' diagnostics.
            span_lint_and_help(
                self.cx,
                BLIND_STORAGE_WRITE,
                expr.span.with_lo(segment.ident.span.lo()),
                BLIND_STORAGE_WRITE_MSG,
                None,
                BLIND_STORAGE_WRITE_HELP,
            );
        }
        state.written_since_read = true;
    }
}

impl<'tcx> LateLintPass<'tcx> for BlindStorageWrite {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: intravisit::FnKind<'tcx>,
        _decl: &'tcx hir::FnDecl<'tcx>,
        body: &'tcx hir::Body<'tcx>,
        _span: rustc_span::Span,
        _def_id: hir::def_id::LocalDefId,
    ) {
        let mut visitor = BlindStorageWriteVisitor {
            cx,
            states: HashMap::new(),
        };
        visitor.visit_expr(body.value);
    }
}

/// Normalises a key expression's source text so equivalent spellings of the
/// same literal compare equal: whitespace is dropped and an integer type suffix
/// (`1i32`, `1u64`, `1_u32`, ...) is stripped from numeric literals. Non-numeric
/// text is preserved verbatim.
fn normalize_key_text(text: &str) -> String {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mut out = String::with_capacity(compact.len());
    let bytes = compact.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            // The digits and the underscores that may group them.
            let mut end = i + 1;
            while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'_') {
                end += 1;
            }
            // `Type::method`-style paths are not suffix-bearing literals.
            let literal = &compact[i..end];
            let trimmed = literal.trim_end_matches('_');
            // A suffix may follow directly (`1i32`) or after an underscore
            // (`1_u32`).
            let mut suffix_end = i + trimmed.len();
            if suffix_end < bytes.len()
                && (bytes[suffix_end].is_ascii_alphabetic() || bytes[suffix_end] == b'_')
            {
                while suffix_end < bytes.len()
                    && (bytes[suffix_end].is_ascii_alphanumeric() || bytes[suffix_end] == b'_')
                {
                    suffix_end += 1;
                }
            }
            let candidate = compact[i + trimmed.len()..suffix_end].trim_start_matches('_');
            if INTEGER_SUFFIXES.contains(&candidate) {
                out.push_str(trimmed);
                i = suffix_end;
            } else {
                out.push_str(literal);
                i = end;
            }
        } else {
            let ch = compact[i..].chars().next().expect("in-bounds char");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// Integer type suffixes strippable from a numeric literal when normalising a
/// storage key.
const INTEGER_SUFFIXES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
];

#[cfg(test)]
mod tests {
    use super::{
        BLIND_STORAGE_WRITE_HELP, BLIND_STORAGE_WRITE_MSG, STORAGE_READ_METHODS,
        STORAGE_WRITE_METHODS, normalize_key_text,
    };

    /// The fixture spells the read-then-write sequence with these methods.
    #[test]
    fn read_and_write_method_sets_are_pinned() {
        assert_eq!(STORAGE_READ_METHODS, ["get", "try_get", "has"]);
        assert_eq!(STORAGE_WRITE_METHODS, ["set", "put", "update"]);
    }

    /// The ui golden pins both strings.
    #[test]
    fn diagnostics_are_pinned() {
        assert_eq!(
            BLIND_STORAGE_WRITE_MSG,
            "blind storage write overwrites a previously written key without reading it back"
        );
        assert!(BLIND_STORAGE_WRITE_HELP.contains("read the previous value"));
    }

    /// `&1`, `& 1` and `&1i32` must name the same key.
    #[test]
    fn key_text_normalisation_ignores_spacing_and_suffixes() {
        assert_eq!(normalize_key_text("&1"), "&1");
        assert_eq!(normalize_key_text("& 1"), "&1");
        assert_eq!(normalize_key_text("&1i32"), "&1");
        assert_eq!(normalize_key_text("&1u64"), "&1");
        assert_eq!(normalize_key_text(" & 1_u32 "), "&1");
        assert_eq!(normalize_key_text("&1_000_000"), "&1_000_000");
    }

    /// Arbitrary expressions normalise only by whitespace; their structure is
    /// preserved.
    #[test]
    fn key_text_normalisation_keeps_structure() {
        assert_eq!(
            normalize_key_text("&symbol_short!(\"k\")"),
            "&symbol_short!(\"k\")"
        );
        assert_eq!(normalize_key_text("&(key + 1)"), "&(key+1)");
        assert_eq!(normalize_key_text("&DATA_KEY"), "&DATA_KEY");
    }
}
