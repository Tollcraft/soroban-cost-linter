//! `repeated_instance_write_same_key` — flags multiple writes to the same
//! instance storage key within a single function body.
//!
//! Each `env.storage().instance().set(&key, ..)` serialises and rewrites the
//! entire instance storage map and pays its own ledger write. Writing twice to
//! the same key in one function pays twice while only the last value survives.

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::LitKind;
use rustc_hir as hir;
use rustc_hir::def::Res;
use rustc_hir::intravisit::{self, Visitor};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use std::collections::HashMap;

use crate::{REPEATED_INSTANCE_WRITE_SAME_KEY, match_soroban_def_path, ty_adt_def};

declare_lint_pass!(RepeatedInstanceWriteSameKey => [REPEATED_INSTANCE_WRITE_SAME_KEY]);

const REPEATED_INSTANCE_WRITE_SAME_KEY_MSG: &str =
    "repeated instance storage write to the same key";
const REPEATED_INSTANCE_WRITE_SAME_KEY_HELP: &str =
    "compute the final value once and write it after the branch or loop that decides it";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum PathKey {
    Hir(hir::HirId),
    Def(rustc_hir::def_id::DefId),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum LiteralKey {
    Str(rustc_span::Symbol),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum KeyIdentity {
    Path(PathKey),
    Lit(LiteralKey),
}

fn extract_key<'tcx>(cx: &LateContext<'tcx>, arg: &'tcx hir::Expr<'tcx>) -> Option<KeyIdentity> {
    let mut inner = arg;
    while let hir::ExprKind::AddrOf(_, _, e) = inner.kind {
        inner = e;
    }

    match inner.kind {
        hir::ExprKind::Lit(lit) => match lit.node {
            LitKind::Str(sym, _) => Some(KeyIdentity::Lit(LiteralKey::Str(sym))),
            _ => None,
        },
        hir::ExprKind::Path(ref qpath) => {
            let res = cx.qpath_res(qpath, inner.hir_id);
            match res {
                Res::Local(hir_id) => Some(KeyIdentity::Path(PathKey::Hir(hir_id))),
                Res::Def(
                    rustc_hir::def::DefKind::Const { .. }
                    | rustc_hir::def::DefKind::AssocConst { .. }
                    | rustc_hir::def::DefKind::Static { .. },
                    def_id,
                ) => {
                    if let Some(local_id) = def_id.as_local() {
                        Some(KeyIdentity::Path(PathKey::Hir(
                            cx.tcx.local_def_id_to_hir_id(local_id),
                        )))
                    } else {
                        Some(KeyIdentity::Path(PathKey::Def(def_id)))
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_instance_storage<'tcx>(cx: &LateContext<'tcx>, receiver: &'tcx hir::Expr<'tcx>) -> bool {
    let receiver_ty = cx.typeck_results().expr_ty(receiver).peel_refs();
    if let Some(adt_def) = ty_adt_def(receiver_ty) {
        match_soroban_def_path(cx, adt_def.did(), &["soroban_sdk", "storage", "Instance"])
    } else {
        false
    }
}

#[derive(Default)]
struct KeyState {
    written: bool,
    reported: bool,
}

struct InstanceWriteVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    key_states: HashMap<KeyIdentity, KeyState>,
}

impl<'a, 'tcx> Visitor<'tcx> for InstanceWriteVisitor<'a, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::Closure(_) = expr.kind {
            // Nested closures are separate function bodies with their own scope.
            return;
        }

        if let hir::ExprKind::MethodCall(path_segment, receiver, args, _span) = expr.kind
            && is_instance_storage(self.cx, receiver)
        {
            let method = path_segment.ident.name.as_str();
            if method == "set" && args.len() >= 2 {
                if let Some(key) = extract_key(self.cx, &args[0]) {
                    let state = self.key_states.entry(key).or_default();
                    if state.written {
                        if !state.reported {
                            span_lint_and_help(
                                self.cx,
                                REPEATED_INSTANCE_WRITE_SAME_KEY,
                                expr.span,
                                REPEATED_INSTANCE_WRITE_SAME_KEY_MSG,
                                None,
                                REPEATED_INSTANCE_WRITE_SAME_KEY_HELP,
                            );
                            state.reported = true;
                        }
                    } else {
                        state.written = true;
                    }
                }
            } else if (method == "get" || method == "try_get" || method == "has")
                && !args.is_empty()
                && let Some(key) = extract_key(self.cx, &args[0])
                && let Some(state) = self.key_states.get_mut(&key)
            {
                state.written = false;
                state.reported = false;
            }
        }

        intravisit::walk_expr(self, expr);
    }
}

impl<'tcx> LateLintPass<'tcx> for RepeatedInstanceWriteSameKey {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: intravisit::FnKind<'tcx>,
        _decl: &'tcx hir::FnDecl<'tcx>,
        body: &'tcx hir::Body<'tcx>,
        _span: rustc_span::Span,
        _def_id: hir::def_id::LocalDefId,
    ) {
        let mut visitor = InstanceWriteVisitor {
            cx,
            key_states: HashMap::new(),
        };
        visitor.visit_expr(body.value);
    }
}
