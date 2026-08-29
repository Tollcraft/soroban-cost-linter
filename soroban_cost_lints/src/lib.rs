#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_hir::{Expr, ExprKind, BinOpKind, UnOp, QPath};
use rustc_lint::{LateContext, LateLintPass, LintContext, LintPass};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

rustc_lint::declare_lint! {
    pub SOROBAN_STORAGE_IN_LOOP,
    Deny,
    "performs a storage read or write inside a loop body"
}

rustc_lint::declare_lint! {
    pub REDUNDANT_ENV_CLONE,
    Warn,
    "clones the Env handle redundantly"
}

rustc_lint::declare_lint! {
    pub UNNECESSARY_HOST_FUNCTION_CALL,
    Warn,
    "calls host functions that could be hoisted or avoided"
}

rustc_lint::declare_lint! {
    pub SOROBAN_REDUNDANT_STORAGE_READ,
    Warn,
    "performs sequential redundant reads or has/get checks on the same key"
}

rustc_lint::declare_lint! {
    pub STORAGE_WRITE_WITHOUT_READ,
    Warn,
    "performs a storage set without any prior get or has in the function"
}

rustc_lint::declare_lint! {
    pub INSTANCE_STORAGE_FOR_UNBOUNDED_DATA,
    Warn,
    "stores unbounded collections like Vec, Map, or Bytes in instance storage"
}

rustc_lint::declare_lint! {
    pub PERSISTENT_READ_WITHOUT_TTL_EXTENSION,
    Warn,
    "reads from persistent storage without extending its TTL in the same function"
}

rustc_lint::declare_lint! {
    pub LOOP_INVARIANT_STORAGE_ACCESS,
    Warn,
    "performs storage access inside a loop with loop-invariant operands"
}

rustc_lint::declare_lint! {
    pub STORAGE_KEY_CONSTRUCTION_IN_LOOP,
    Warn,
    "constructs storage keys inside loop bodies where the key is invariant"
}

rustc_lint::declare_lint! {
    pub BYTES_APPEND_IN_LOOP,
    Warn,
    "appends to Bytes or Vec inside loop bodies causing repeated host reallocations"
}

rustc_lint::declare_lint! {
    pub UNBOUNDED_INPUT_LOOP,
    Warn,
    "loops with iteration count derived from untrusted input performing storage writes"
}

rustc_lint::declare_lint! {
    pub UNNECESSARY_STRING_TO_BYTES,
    Warn,
    "performs unnecessary string to bytes conversion"
}

rustc_lint::declare_lint! {
    pub UNNECESSARY_HOST_FUNCTION_CALL_LEGACY,
    Warn,
    "legacy unnecessary host function call"
}

rustc_lint::declare_lint! {
    pub MAP_INSERT_IN_LOOP,
    Warn,
    "inserts into Map inside a loop"
}

rustc_lint::declare_lint! {
    pub INEFFICIENT_BYTES_CONCAT,
    Warn,
    "inefficient bytes concatenation"
}

rustc_lint::declare_lint! {
    pub CONTRACT_CALL_IN_LOOP,
    Warn,
    "performs contract call inside loop"
}

rustc_lint::declare_lint! {
    pub EXTEND_TTL_IN_LOOP,
    Warn,
    "extends ttl inside loop"
}

rustc_lint::declare_lint! {
    pub FORMATTED_PANIC_PAYLOAD,
    Warn,
    "formatted panic payload"
}

rustc_lint::declare_lint! {
    pub LINEAR_SCAN_IN_LOOP,
    Warn,
    "linear scan inside loop"
}

rustc_lint::declare_lint! {
    pub REQUIRE_AUTH_IN_LOOP,
    Warn,
    "requires auth inside loop"
}

rustc_lint::declare_lint! {
    pub SIGNATURE_VERIFICATION_IN_LOOP,
    Warn,
    "signature verification inside loop"
}

rustc_lint::declare_lint! {
    pub SYMBOL_KEY_BOUNDARY,
    Warn,
    "symbol key boundary"
}

rustc_lint::declare_lint! {
    pub SYMBOL_KEY_ENUM_STORAGE,
    Warn,
    "symbol key enum storage"
}

rustc_lint::declare_lint! {
    pub SYMBOL_KEY_EVENT_TOPICS,
    Warn,
    "symbol key event topics"
}

rustc_lint::declare_lint! {
    pub SYMBOL_NEW_FOR_SHORT_LITERAL,
    Warn,
    "uses Symbol::new for short literal"
}

rustc_lint::declare_lint! {
    pub UNBOUNDED_RECURSION,
    Warn,
    "unbounded recursion"
}

rustc_lint::declare_lint! {
    pub UNWRAP_ON_STORAGE_GET,
    Warn,
    "unwraps on storage get"
}

rustc_lint::declare_lint! {
    pub VEC_WHERE_SLICE_COULD_BE_USED,
    Warn,
    "uses Vec where slice could be used"
}

rustc_lint::declare_lint! {
    pub SOROBAN_INEFFICIENT_BYTES_CONCAT,
    Warn,
    "soroban inefficient bytes concat"
}

rustc_lint::declare_lint! {
    pub U128_WHERE_U64_SUFFICES,
    Warn,
    "uses 128-bit arithmetic where 64 bits would suffice, which is extremely expensive on wasm32"
}

pub struct SorobanCostLints;

#[derive(Debug, Clone, Copy)]
enum LintCategory {
    Storage,
    Compute,
    Memory,
    Host,
    Security,
}

impl LintCategory {
    fn as_str(&self) -> &'static str {
        match self => {
            LintCategory::Storage => "Storage",
            LintCategory::Compute => "Compute",
            LintCategory::Memory => "Memory",
            LintCategory::Host => "Host",
            LintCategory::Security => "Security",
        }
    }
}

pub struct LintMeta {
    pub name: &'static str,
    pub category: LintCategory,
    pub description: &'static str,
    pub rationale: &'static str,
}

pub const LINT_METADATA: &[LintMeta] = &[
    LintMeta {
        name: "soroban_storage_in_loop",
        category: LintCategory::Storage,
        description: "Performs a storage read or write inside a loop body",
        rationale: "Storage operations are extremely expensive in Soroban; performing them inside loops can quickly exhaust budget.",
    },
    LintMeta {
        name: "redundant_env_clone",
        category: LintCategory::Host,
        description: "Clones the Env handle redundantly",
        rationale: "Env is a cheap handle and cloning it repeatedly adds overhead.",
    },
    LintMeta {
        name: "unnecessary_host_function_call",
        category: LintCategory::Host,
        description: "Calls host functions that could be hoisted or avoided",
        rationale: "Host function calls cross the Wasm boundary and consume CPU.",
    },
    LintMeta {
        name: "soroban_redundant_storage_read",
        category: LintCategory::Storage,
        description: "Performs sequential redundant reads or has/get checks on the same key",
        rationale: "Repeatedly reading the same storage entry without intervening writes wastes ledger IO and CPU.",
    },
    LintMeta {
        name: "storage_write_without_read",
        category: LintCategory::Storage,
        description: "Performs a storage set without any prior get or has in the function",
        rationale: "Blind writes can overwrite state accidentally and lack update guards.",
    },
    LintMeta {
        name: "instance_storage_for_unbounded_data",
        category: LintCategory::Storage,
        description: "Stores unbounded collections like Vec, Map, or Bytes in instance storage",
        rationale: "Instance storage has a strict 64KB footprint limit and shares contract TTL.",
    },
    LintMeta {
        name: "persistent_read_without_ttl_extension",
        category: LintCategory::Storage,
        description: "Reads from persistent storage without extending its TTL in the same function",
        rationale: "Persistent entries expire if their TTL is not extended, risking archival.",
    },
    LintMeta {
        name: "loop_invariant_storage_access",
        category: LintCategory::Storage,
        description: "Performs storage access inside a loop with loop-invariant operands",
        rationale: "Invariants should be hoisted outside the loop.",
    },
    LintMeta {
        name: "storage_key_construction_in_loop",
        category: LintCategory::Storage,
        description: "Constructs storage keys inside loop bodies where the key is invariant",
        rationale: "Key construction inside loops wastes CPU.",
    },
    LintMeta {
        name: "bytes_append_in_loop",
        category: LintCategory::Memory,
        description: "Appends to Bytes or Vec inside loop bodies causing repeated host reallocations",
        rationale: "Incremental host appending causes repeated allocations.",
    },
    LintMeta {
        name: "unbounded_input_loop",
        category: LintCategory::Compute,
        description: "Loops with iteration count derived from untrusted input performing storage writes",
        rationale: "Unbounded loops over untrusted input are a denial-of-service vector.",
    },
    LintMeta {
        name: "map_insert_in_loop",
        category: LintCategory::Memory,
        description: "Inserts into Map inside a loop",
        rationale: "Repeated insertions inside loops can be inefficient.",
    },
    LintMeta {
        name: "inefficient_bytes_concat",
        category: LintCategory::Memory,
        description: "Inefficient bytes concatenation",
        rationale: "Inefficient concatenation wastes memory and CPU.",
    },
    LintMeta {
        name: "contract_call_in_loop",
        category: LintCategory::Host,
        description: "Performs contract call inside loop",
        rationale: "Cross-contract calls in loops are extremely expensive.",
    },
    LintMeta {
        name: "extend_ttl_in_loop",
        category: LintCategory::Storage,
        description: "Extends ttl inside loop",
        rationale: "Extending TTL per iteration wastes CPU.",
    },
    LintMeta {
        name: "formatted_panic_payload",
        category: LintCategory::Compute,
        description: "Formatted panic payload",
        rationale: "String formatting in panics consumes extra Wasm memory and CPU.",
    },
    LintMeta {
        name: "linear_scan_in_loop",
        category: LintCategory::Compute,
        description: "Linear scan inside loop",
        rationale: "O(N^2) scans inside loops degrade performance.",
    },
    LintMeta {
        name: "require_auth_in_loop",
        category: LintCategory::Security,
        description: "Requires auth inside loop",
        rationale: "Authorization checks are expensive host calls.",
    },
    LintMeta {
        name: "signature_verification_in_loop",
        category: LintCategory::Security,
        description: "Signature verification inside loop",
        rationale: "Crypto checks in loops are extremely expensive.",
    },
    LintMeta {
        name: "symbol_key_boundary",
        category: LintCategory::Storage,
        description: "Symbol key boundary",
        rationale: "Symbol keys should follow naming boundaries.",
    },
    LintMeta {
        name: "symbol_key_enum_storage",
        category: LintCategory::Storage,
        description: "Symbol key enum storage",
        rationale: "Enum storage keys should be optimized.",
    },
    LintMeta {
        name: "symbol_key_event_topics",
        category: LintCategory::Storage,
        description: "Symbol key event topics",
        rationale: "Event topics should use efficient keys.",
    },
    LintMeta {
        name: "symbol_new_for_short_literal",
        category: LintCategory::Compute,
        description: "Uses Symbol::new for short literal",
        rationale: "Short literals should use symbol_short! macro.",
    },
    LintMeta {
        name: "unbounded_recursion",
        category: LintCategory::Compute,
        description: "Unbounded recursion",
        rationale: "Recursion without provable bounds can overflow stack.",
    },
    LintMeta {
        name: "unwrap_on_storage_get",
        category: LintCategory::Storage,
        description: "Unwraps on storage get",
        rationale: "Unwrap on storage get can panic unexpectedly on missing state.",
    },
    LintMeta {
        name: "vec_where_slice_could_be_used",
        category: LintCategory::Memory,
        description: "Uses Vec where slice could be used",
        rationale: "Slices avoid host-side allocations.",
    },
    LintMeta {
        name: "soroban_inefficient_bytes_concat",
        category: LintCategory::Memory,
        description: "Soroban inefficient bytes concat",
        rationale: "Inefficient bytes concat wastes memory.",
    },
    LintMeta {
        name: "u128_where_u64_suffices",
        category: LintCategory::Compute,
        description: "Uses 128-bit arithmetic where 64 bits would suffice",
        rationale: "wasm32 is a 32-bit target. 128-bit arithmetic is heavily emulated and extremely expensive; values provably within 64 bits should use u64/i64.",
    },
    LintMetadata {
        lint: PERSISTENT_STORAGE_FOR_EPHEMERAL_DATA,
        category: LintCategory::EntryLifecycle,
    },
];

impl LintPass for SorobanCostLints {
    fn name(&self) -> &'static str {
        "SorobanCostLints"
    }
}

impl<'tcx> LateLintPass<'tcx> for SorobanCostLints {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        check_u128_arithmetic(cx, expr);
    }
}

fn check_u128_arithmetic<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
    let ty = cx.typeck_results().expr_ty(expr);
    if is_u128_or_i128(ty) {
        if let ExprKind::Binary(op, lhs, rhs) = expr.kind {
            if is_arithmetic_op(op.node) {
                if is_provably_within_64_bits(cx, lhs) && is_provably_within_64_bits(cx, rhs) {
                    cx.span_lint(
                        U128_WHERE_U64_SUFFICES,
                        expr.span,
                        |diag| {
                            diag.primary_message(
                                "128-bit arithmetic used where 64 bits would suffice; wasm32 is a 32-bit target and emulated 128-bit operations are significantly more expensive."
                            );
                        },
                    );
                }
            }
        } else if let ExprKind::AssignOp(op, lhs, rhs) = expr.kind {
            if is_arithmetic_op(op.node) {
                if is_provably_within_64_bits(cx, lhs) && is_provably_within_64_bits(cx, rhs) {
                    cx.span_lint(
                        U128_WHERE_U64_SUFFICES,
                        expr.span,
                        |diag| {
                            diag.primary_message(
                                "128-bit arithmetic used where 64 bits would suffice; wasm32 is a 32-bit target and emulated 128-bit operations are significantly more expensive."
                            );
                        },
                    );
                }
            }
        }
    }
}

fn is_u128_or_i128(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Int(ty::IntTy::I128) | ty::Uint(ty::UintTy::U128))
}

fn is_arithmetic_op(op: BinOpKind) -> bool {
    matches!(
        op,
        BinOpKind::Add
            | BinOpKind::Sub
            | BinOpKind::Mul
            | BinOpKind::Div
            | BinOpKind::Rem
            | BinOpKind::Shl
            | BinOpKind::Shr
    )
}

fn is_provably_within_64_bits<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr);
    // If the expression's own type is already a u32, i32, u64, i64, usize, isize, etc., it's within 64 bits.
    if let ty::Int(int_ty) = ty.kind() {
        if !matches!(int_ty, ty::IntTy::I128) {
            return true;
        }
    }
    if let ty::Uint(uint_ty) = ty.kind() {
        if !matches!(uint_ty, ty::UintTy::U128) {
            return true;
        }
    }

    match expr.kind {
        ExprKind::Lit(lit) => {
            match lit.node {
                rustc_ast::ast::LitKind::Int(val, _) => {
                    // Fits in i64/u64 max
                    val.get() <= u64::MAX as u128
                }
                _ => false,
            }
        }
        ExprKind::Cast(inner, _target_ty) => {
            let inner_ty = cx.typeck_results().expr_ty(inner);
            if let ty::Int(it) = inner_ty.kind() {
                if !matches!(it, ty::IntTy::I128) { return true; }
            }
            if let ty::Uint(ut) = inner_ty.kind() {
                if !matches!(ut, ty::UintTy::U128) { return true; }
            }
            is_provably_within_64_bits(cx, inner)
        }
        ExprKind::MethodCall(_segment, receiver, args, _span) => {
            // e.g. len() on a collection or u32 length conversion
            let method_name = _segment.ident.as_str();
            if method_name == "len" || method_name == "min" || method_name == "max" {
                return true;
            }
            // check receiver or args
            is_provably_within_64_bits(cx, receiver)
        }
        ExprKind::Binary(op, lhs, rhs) => {
            if matches!(op.node, BinOpKind::And | BinOpKind::Or | BinOpKind::BitAnd | BinOpKind::BitOr) {
                return is_provably_within_64_bits(cx, lhs) || is_provably_within_64_bits(cx, rhs);
            }
            false
        }
        _ => false,
    }
}

#[no_mangle আনন্দের]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[
        SOROBAN_STORAGE_IN_LOOP,
        REDUNDANT_ENV_CLONE,
        UNNECESSARY_HOST_FUNCTION_CALL,
        SOROBAN_REDUNDANT_STORAGE_READ,
        STORAGE_WRITE_WITHOUT_READ,
        INSTANCE_STORAGE_FOR_UNBOUNDED_DATA,
        PERSISTENT_READ_WITHOUT_TTL_EXTENSION,
        LOOP_INVARIANT_STORAGE_ACCESS,
        STORAGE_KEY_CONSTRUCTION_IN_LOOP,
        BYTES_APPEND_IN_LOOP,
        UNBOUNDED_INPUT_LOOP,
        UNNECESSARY_STRING_TO_BYTES,
        UNNECESSARY_HOST_FUNCTION_CALL_LEGACY,
        MAP_INSERT_IN_LOOP,
        INEFFICIENT_BYTES_CONCAT,
        CONTRACT_CALL_IN_LOOP,
        EXTEND_TTL_IN_LOOP,
        FORMATTED_PANIC_PAYLOAD,
        LINEAR_SCAN_IN_LOOP,
        REQUIRE_AUTH_IN_LOOP,
        SIGNATURE_VERIFICATION_IN_LOOP,
        SYMBOL_KEY_BOUNDARY,
        SYMBOL_KEY_ENUM_STORAGE,
        SYMBOL_KEY_EVENT_TOPICS,
        SYMBOL_NEW_FOR_SHORT_LITERAL,
        UNBOUNDED_RECURSION,
        UNWRAP_ON_STORAGE_GET,
        VEC_WHERE_SLICE_COULD_BE_USED,
        SOROBAN_INEFFICIENT_BYTES_CONCAT,
        U128_WHERE_U64_SUFFICES,
    ]);
    lint_store.register_group(
        "soroban_cost_lints",
        Some(rustc_span::Symbol::intern("soroban_cost_lints_group")),
        vec![
            SOROBAN_STORAGE_IN_LOOP,
            REDUNDANT_ENV_CLONE,
            UNNECESSARY_HOST_FUNCTION_CALL,
            SOROBAN_REDUNDANT_STORAGE_READ,
            STORAGE_WRITE_WITHOUT_READ,
            INSTANCE_STORAGE_FOR_UNBOUNDED_DATA,
            PERSISTENT_READ_WITHOUT_TTL_EXTENSION,
            LOOP_INVARIANT_STORAGE_ACCESS,
            STORAGE_KEY_CONSTRUCTION_IN_LOOP,
            BYTES_APPEND_IN_LOOP,
            UNBOUNDED_INPUT_LOOP,
            UNNECESSARY_STRING_TO_BYTES,
            UNNECESSARY_HOST_FUNCTION_CALL_LEGACY,
            MAP_INSERT_IN_LOOP,
            INEFFICIENT_BYTES_CONCAT,
            CONTRACT_CALL_IN_LOOP,
            EXTEND_TTL_IN_LOOP,
            FORMATTED_PANIC_PAYLOAD,
            LINEAR_SCAN_IN_LOOP,
            REQUIRE_AUTH_IN_LOOP,
            SIGNATURE_VERIFICATION_IN_LOOP,
            SYMBOL_KEY_BOUNDARY,
            SYMBOL_KEY_ENUM_STORAGE,
            SYMBOL_KEY_EVENT_TOPICS,
            SYMBOL_NEW_FOR_SHORT_LITERAL,
            UNBOUNDED_RECURSION,
            UNWRAP_ON_STORAGE_GET,
        ],
    );
}


// =======================================================================
// collection_len_in_loop_condition - Lint
// =======================================================================

rustc_session::declare_lint! {
    /// ### What it does
    /// Detects `.len()` calls on Soroban collections inside a `while` loop condition
    /// when the collection is not mutated within the loop.
    pub COLLECTION_LEN_IN_LOOP_CONDITION,
    Warn,
    "collection len() called in a loop condition without mutation"
}

pub struct CollectionLenInLoopCondition;
rustc_session::impl_lint_pass!(CollectionLenInLoopCondition => [COLLECTION_LEN_IN_LOOP_CONDITION]);

impl<'tcx> LateLintPass<'tcx> for CollectionLenInLoopCondition {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::MethodCall(path_segment, receiver, _args, _span) = expr.kind {
            if path_segment.ident.name.as_str() == "len" {
                let receiver_ty = cx.typeck_results().expr_ty(receiver);
                let peeled_ty = receiver_ty.peel_refs();
                
                if let rustc_middle::ty::Adt(adt_def, _) = peeled_ty.kind() {
                    let did = adt_def.did();
                    
                    if match_soroban_def_path(cx, did, &["soroban_sdk", "vec", "Vec"]) ||
                       match_soroban_def_path(cx, did, &["soroban_sdk", "map", "Map"]) ||
                       match_soroban_def_path(cx, did, &["soroban_sdk", "bytes", "Bytes"]) ||
                       match_soroban_def_path(cx, did, &["soroban_sdk", "string", "String"]) 
                    {
                        if let Some(loop_expr) = enclosing_loop(cx, expr) {
                            if let hir::ExprKind::Loop(_block, _label, hir::LoopSource::While, _) = loop_expr.kind {
                                if !depends_on_loop_state(cx, loop_expr, expr) {
                                    clippy_utils::diagnostics::span_lint_and_help(
                                        cx,
                                        COLLECTION_LEN_IN_LOOP_CONDITION,
                                        expr.span,
                                        "collection len() called in a loop condition without mutation",
                                        None,
                                        "hoist/bind the collection's length into a local variable before the loop starts, and compare against that local in the while condition instead of calling .len() each iteration.",
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

