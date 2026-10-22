//! Flags repeated construction of a Soroban `String` from bytes inside a loop.
//!
//! Soroban SDK 28 does not expose `String::append` or `String + String`. A
//! supported way to rebuild a growing string is `String::from_bytes`, which
//! copies and converts the byte slice into a host-backed string each time.

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::def::Res;
use rustc_hir::intravisit::{Visitor, walk_expr, walk_stmt};
use rustc_hir::{Expr, ExprKind, HirId, PatKind, QPath, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::{STRING_CONCAT_IN_LOOP, enclosing_loop, match_soroban_def_path};

declare_lint_pass!(StringConcatInLoop => [STRING_CONCAT_IN_LOOP]);

const BUFFER_GROWTH_METHODS: &[&str] = &["push", "extend", "extend_from_slice", "append", "insert"];

impl<'tcx> LateLintPass<'tcx> for StringConcatInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Call(callee, _args) = expr.kind
            && let ExprKind::Path(ref qpath) = callee.kind
            && let Some(def_id) = cx.qpath_res(qpath, callee.hir_id).opt_def_id()
            && match_soroban_def_path(cx, def_id, &["soroban_sdk", "String", "from_bytes"])
            && let Some(loop_expr) = enclosing_loop(cx, expr)
            && let ExprKind::Call(_, args) = expr.kind
            && let Some(bytes) = args.get(1)
            && let Some(binding) = buffer_binding(bytes)
            && buffer_grows_before_call(loop_expr, expr.hir_id, binding)
        {
            span_lint_and_help(
                cx,
                STRING_CONCAT_IN_LOOP,
                expr.span,
                "repeatedly constructing a Soroban String from bytes inside a loop",
                None,
                "build the byte buffer inside the loop and construct the String once afterwards",
            );
        }
    }
}

fn buffer_binding(expr: &Expr<'_>) -> Option<HirId> {
    match expr.kind {
        ExprKind::AddrOf(_, _, inner) => buffer_binding(inner),
        ExprKind::MethodCall(segment, receiver, args, _)
            if args.is_empty() && matches!(segment.ident.as_str(), "as_slice" | "as_ref") =>
        {
            buffer_binding(receiver)
        }
        ExprKind::Path(QPath::Resolved(None, path)) => match path.res {
            Res::Local(binding) => Some(binding),
            _ => None,
        },
        _ => None,
    }
}

fn buffer_grows_before_call(loop_expr: &Expr<'_>, call_id: HirId, binding: HirId) -> bool {
    let mut declarations = LoopBindingVisitor {
        binding,
        declared_in_loop: false,
    };
    declarations.visit_expr(loop_expr);
    if declarations.declared_in_loop {
        return false;
    }

    let mut growth = BufferGrowthVisitor {
        binding,
        call_id,
        grew_since_reset: false,
        reset_seen: false,
        reached_call: false,
    };
    growth.visit_expr(loop_expr);
    growth.reached_call && growth.grew_since_reset
}

struct LoopBindingVisitor {
    binding: HirId,
    declared_in_loop: bool,
}

impl<'hir> Visitor<'hir> for LoopBindingVisitor {
    fn visit_stmt(&mut self, stmt: &'hir rustc_hir::Stmt<'hir>) {
        if let StmtKind::Let(local) = stmt.kind
            && let PatKind::Binding(_, id, _, _) = local.pat.kind
            && id == self.binding
        {
            self.declared_in_loop = true;
            return;
        }
        walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'hir Expr<'hir>) {
        if matches!(expr.kind, ExprKind::Closure(..)) {
            return;
        }
        walk_expr(self, expr);
    }
}

struct BufferGrowthVisitor {
    binding: HirId,
    call_id: HirId,
    grew_since_reset: bool,
    reset_seen: bool,
    reached_call: bool,
}

impl BufferGrowthVisitor {
    fn receiver_is_buffer(&self, receiver: &Expr<'_>) -> bool {
        buffer_binding(receiver) == Some(self.binding)
    }

    fn target_is_buffer(&self, target: &Expr<'_>) -> bool {
        matches!(target.kind, ExprKind::Path(QPath::Resolved(None, path))
            if matches!(path.res, Res::Local(id) if id == self.binding))
    }
}

impl<'hir> Visitor<'hir> for BufferGrowthVisitor {
    fn visit_expr(&mut self, expr: &'hir Expr<'hir>) {
        if self.reached_call || matches!(expr.kind, ExprKind::Closure(..)) {
            return;
        }
        if expr.hir_id == self.call_id {
            self.reached_call = true;
            return;
        }
        match expr.kind {
            ExprKind::MethodCall(segment, receiver, _, _) if self.receiver_is_buffer(receiver) => {
                match segment.ident.as_str() {
                    "clear" => {
                        self.grew_since_reset = false;
                        self.reset_seen = true;
                    }
                    method if BUFFER_GROWTH_METHODS.contains(&method) => {
                        if !self.reset_seen {
                            self.grew_since_reset = true;
                        }
                    }
                    _ => {}
                }
            }
            ExprKind::Assign(target, _, _) if self.target_is_buffer(target) => {
                self.grew_since_reset = false;
                self.reset_seen = true;
            }
            ExprKind::AssignOp(op, target, _) if self.target_is_buffer(target) => {
                if op.node == rustc_hir::BinOpKind::Add {
                    if !self.reset_seen {
                        self.grew_since_reset = true;
                    }
                } else {
                    self.grew_since_reset = false;
                    self.reset_seen = true;
                }
            }
            _ => {}
        }
        walk_expr(self, expr);
    }
}
