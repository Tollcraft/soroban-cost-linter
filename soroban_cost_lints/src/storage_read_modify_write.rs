use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Block, Expr, ExprKind, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass, Body};
use rustc_session::{declare_lint, declare_lint_pass};
use std::collections::HashMap;

declare_lint! {
    pub STORAGE_READ_MODIFY_WRITE,
    Warn,
    "performs two or more read-modify-write cycles on the same storage key"
}

declare_lint_pass!(StorageReadModifyWrite => [STORAGE_READ_MODIFY_WRITE]);

/// A storage key identified by its source text and namespace.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct StorageKey {
    text: String,
    namespace: String,
}

/// State machine per key tracking read-modify-write cycles.
#[derive(Debug, Clone, Default)]
struct KeyState {
    /// Number of completed read-modify-write cycles seen so far.
    cycle_count: usize,
    /// Whether we've seen a read without a subsequent write (pending read).
    has_pending_read: bool,
}

/// A storage operation we recognise.
enum StorageOp<'a> {
    Read(&'a Expr<'a>),
    Write(rustc_span::Span),
}

impl<'tcx> LateLintPass<'tcx> for StorageReadModifyWrite {
    fn check_body(&mut self, cx: &LateContext<'tcx>, body: &Body<'tcx>) {
        let expr = body.value;
        if let ExprKind::Block(block, _) = expr.kind {
            check_block(cx, block);
        }
    }
}

/// Walk the top-level statements of a block and track read-modify-write
/// cycles per storage key.
fn check_block<'tcx>(cx: &LateContext<'tcx>, block: &Block<'tcx>) {
    let mut key_states: HashMap<StorageKey, KeyState> = HashMap::new();

    for stmt in block.stmts {
        match stmt.kind {
            StmtKind::Let(local) => {
                if let Some(init) = local.init {
                    process_expr(cx, init, &mut key_states);
                }
            }
            StmtKind::Item(_) => {}
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => {
                process_expr(cx, expr, &mut key_states);
            }
        }
    }

    if let Some(trailing) = block.expr {
        process_expr(cx, trailing, &mut key_states);
    }
}

/// Process a top-level expression. We care about two kinds:
///   1. Storage reads/writes (get/has/set on Instance/Persistent/Temporary)
///   2. Function or method calls that could touch storage
fn process_expr<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    key_states: &mut HashMap<StorageKey, KeyState>,
) {
    // Is this a direct storage read or write?
    if let Some(op) = extract_storage_op(cx, expr) {
        match op {
            StorageOp::Read(key_expr) => {
                let key = make_key(cx, key_expr);
                let state = key_states.entry(key).or_default();
                state.has_pending_read = true;
            }
            StorageOp::Write(span) => {
                // The key for a set() call is the first argument.
                if let ExprKind::MethodCall(_, _, args, _) = expr.kind {
                    if let Some(key_arg) = args.first() {
                        let key = make_key(cx, key_arg);
                        let state = key_states.entry(key).or_default();
                        if state.has_pending_read {
                            state.cycle_count += 1;
                            state.has_pending_read = false;

                            if state.cycle_count >= 2 {
                                span_lint_and_help(
                                    cx,
                                    STORAGE_READ_MODIFY_WRITE,
                                    span,
                                    "storage read-modify-write cycle: this key was already read, modified, and written once; repeating the cycle wastes storage operations",
                                    None,
                                    "reuse the value from the first cycle instead of re-reading and re-writing the same key",
                                );
                            }
                        }
                    }
                }
            }
        }
        return;
    }

    // Not a direct storage op — could this expression call something
    // that touches storage?
    if is_potential_storage_call(cx, expr) {
        // Conservative: reset all tracked keys.
        for state in key_states.values_mut() {
            state.cycle_count = 0;
            state.has_pending_read = false;
        }
    }
}

fn make_key<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> StorageKey {
    let namespace = extract_storage_namespace_from_context(cx, expr);
    StorageKey {
        text: format!("{}", cx.sess().source_map().span_to_string(expr.span)),
        namespace,
    }
}

/// Infer the storage namespace from the context of a key expression.
fn extract_storage_namespace_from_context<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
) -> String {
    let hir = cx.tcx.hir();
    let parent_id = hir.parent_id(expr.hir_id);
    if let Some(rustc_hir::Node::Expr(parent_expr)) = hir.find(parent_id) {
        if let ExprKind::MethodCall(path, receiver, _, _) = parent_expr.kind {
            let method = path.ident.as_str();
            if method == "get" || method == "has" || method == "set" {
                return infer_namespace_from_receiver(cx, receiver);
            }
        }
    }
    "unknown".to_string()
}

fn infer_namespace_from_receiver<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> String {
    let ty = cx.typeck_results().expr_ty(expr);
    let ty_str = format!("{:?}", ty);
    if ty_str.contains("Instance") {
        "instance".to_string()
    } else if ty_str.contains("Persistent") {
        "persistent".to_string()
    } else if ty_str.contains("Temporary") {
        "temporary".to_string()
    } else {
        if let ExprKind::MethodCall(path, _, _, _) = expr.kind {
            let name = path.ident.as_str();
            if name == "instance" || name == "persistent" || name == "temporary" {
                return name.to_string();
            }
        }
        "unknown".to_string()
    }
}

/// Try to extract a storage read or write from an expression.
fn extract_storage_op<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
) -> Option<StorageOp<'tcx>> {
    if let ExprKind::MethodCall(path, receiver, args, _) = expr.kind {
        let method = path.ident.as_str();
        match method {
            "get" | "has" => {
                if is_storage_receiver(cx, receiver) {
                    if let Some(key) = args.first() {
                        return Some(StorageOp::Read(key));
                    }
                }
            }
            "set" => {
                if is_storage_receiver(cx, receiver) {
                    return Some(StorageOp::Write(expr.span));
                }
            }
            _ => {}
        }
    }
    None
}

fn is_storage_receiver<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr);
    let s = format!("{:?}", ty);
    s.contains("Instance") || s.contains("Persistent") || s.contains("Temporary")
}

/// Conservatively determine whether an expression might call code
/// that touches storage.
fn is_potential_storage_call<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> bool {
    match expr.kind {
        ExprKind::Call(_, _) => true,
        ExprKind::MethodCall(_, receiver, _, _) => !is_primitive_receiver(cx, receiver),
        ExprKind::If(_, then, else_expr) => {
            let then_has = block_has_non_storage_call(cx, then);
            let else_has = else_expr.map_or(false, |e| is_potential_storage_call(cx, e));
            then_has || else_has
        }
        ExprKind::Block(block, _) => block_has_non_storage_call(cx, block),
        _ => false,
    }
}

fn block_has_non_storage_call<'tcx>(cx: &LateContext<'tcx>, block: &Block<'tcx>) -> bool {
    for stmt in block.stmts {
        match stmt.kind {
            StmtKind::Let(local) => {
                if let Some(init) = local.init {
                    if is_potential_storage_call(cx, init) {
                        return true;
                    }
                }
            }
            StmtKind::Expr(e) | StmtKind::Semi(e) => {
                if is_potential_storage_call(cx, e) {
                    return true;
                }
            }
            _ => {}
        }
    }
    block
        .expr
        .as_ref()
        .map_or(false, |e| is_potential_storage_call(cx, e))
}

/// Check whether the receiver is a known-pure primitive type.
fn is_primitive_receiver<'tcx>(_cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> bool {
    matches!(expr.kind, ExprKind::Lit(_))
}
