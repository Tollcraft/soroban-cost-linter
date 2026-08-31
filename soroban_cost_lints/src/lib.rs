#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_hir::{Crate, Expr, ExprKind, BinOpKind, UnOp, QPath};
use rustc_lint::{LateContext, LateLintPass, LintContext, LintPass};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use std::collections::HashSet;

mod discarded_storage_read;
mod storage_read_modify_write;

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
    pub DISCARDED_STORAGE_READ,
    Warn,
    "reads from storage whose result is never used"
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

rustc_lint::declare_lint! {
    pub STORAGE_READ_MODIFY_WRITE,
    Warn,
    "performs two or more read-modify-write cycles on the same storage key"
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
        match self {
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
        name: "discarded_storage_read",
        category: LintCategory::Storage,
        description: "Reads from storage whose result is never used",
        rationale: "Storage reads are among the most expensive operations in Soroban; reading data without using it wastes ledger bandwidth and gas with zero behavioral purpose.",
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
        rationale: "Reallocating memory inside loops is inefficient.",
    },
    LintMeta {
        name: "unbounded_input_loop",
        category: LintCategory::Compute,
        description: "Loops with iteration count derived from untrusted input performing storage writes",
        rationale: "Loops controlled by untrusted input can cause excessive execution cost.",
    },
    LintMeta {
        name: "unnecessary_string_to_bytes",
        category: LintCategory::Memory,
        description: "Performs unnecessary string to bytes conversion",
        rationale: "Unnecessary string-to-bytes conversions waste CPU cycles.",
    },
    LintMeta {
        name: "map_insert_in_loop",
        category: LintCategory::Compute,
        description: "Inserts into Map inside a loop",
        rationale: "Map insertions inside loops can be expensive.",
    },
    LintMeta {
        name: "inefficient_bytes_concat",
        category: LintCategory::Memory,
        description: "Inefficient bytes concatenation",
        rationale: "Concatenating bytes inefficiently leads to high memory overhead.",
    },
    LintMeta {
        name: "contract_call_in_loop",
        category: LintCategory::Compute,
        description: "Performs contract call inside loop",
        rationale: "Contract calls inside loops multiply cross-contract overhead.",
    },
    LintMeta {
        name: "extend_ttl_in_loop",
        category: LintCategory::Storage,
        description: "Extends ttl inside loop",
        rationale: "Extending TTL inside loops is redundant and costly.",
    },
    LintMeta {
        name: "formatted_panic_payload",
        category: LintCategory::Compute,
        description: "Formatted panic payload",
        rationale: "Formatted panic strings consume unnecessary memory and CPU.",
    },
    LintMeta {
        name: "linear_scan_in_loop",
        category: LintCategory::Compute,
        description: "Linear scan inside loop",
        rationale: "Linear scans inside loops degrade algorithmic complexity.",
    },
    LintMeta {
        name: "require_auth_in_loop",
        category: LintCategory::Security,
        description: "Requires auth inside loop",
        rationale: "Authorization checks inside loops repeat expensive signature validations.",
    },
    LintMeta {
        name: "signature_verification_in_loop",
        category: LintCategory::Security,
        description: "Signature verification inside loop",
        rationale: "Signature verifications are computationally heavy.",
    },
    LintMeta {
        name: "symbol_key_boundary",
        category: LintCategory::Storage,
        description: "Symbol key boundary",
        rationale: "Ensure symbol keys respect length limits and conventions.",
    },
    LintMeta {
        name: "symbol_key_enum_storage",
        category: LintCategory::Storage,
        description: "Symbol key enum storage",
        rationale: "Optimizes enum storage keys.",
    },
    LintMeta {
        name: "symbol_key_event_topics",
        category: LintCategory::Host,
        description: "Symbol key event topics",
        rationale: "Optimizes event topic symbol keys.",
    },
    LintMeta {
        name: "symbol_new_for_short_literal",
        category: LintCategory::Compute,
        description: "Uses Symbol::new for short literal",
        rationale: "Use symbol_short! macro instead of Symbol::new for short literals.",
    },
    LintMeta {
        name: "unbounded_recursion",
        category: LintCategory::Compute,
        description: "Unbounded recursion",
        rationale: "Recursion without bounds can overflow stack and exhaust resources.",
    },
    LintMeta {
        name: "unwrap_on_storage_get",
        category: LintCategory::Storage,
        description: "Unwraps on storage get",
        rationale: "Unwrapping optional storage gets can cause unexpected contract panics when keys are absent.",
    },
    LintMeta {
        name: "vec_where_slice_could_be_used",
        category: LintCategory::Memory,
        description: "Uses Vec where slice could be used",
        rationale: "Slices avoid unnecessary heap allocations.",
    },
    LintMeta {
        name: "soroban_inefficient_bytes_concat",
        category: LintCategory::Memory,
        description: "Soroban inefficient bytes concat",
        rationale: "Inefficient bytes concatenation in Soroban environment.",
    },
    LintMeta {
        name: "u128_where_u64_suffices",
        category: LintCategory::Compute,
        description: "Uses 128-bit arithmetic where 64 bits would suffice, which is extremely expensive on wasm32",
        rationale: "wasm32 lacks native 128-bit integer instructions; emulating them is very slow.",
    },
    LintMeta {
        name: "storage_read_modify_write",
        category: LintCategory::Storage,
        description: "Performs two or more read-modify-write cycles on the same storage key",
        rationale: "Each extra cycle is a full metered read plus a full metered write for a value that never left the host between them. On a balance update touched by three helpers, that is six storage operations where two would do.",
    },
];

dylint_lint_impl! {
    SorobanCostLints,
    [
        SOROBAN_STORAGE_IN_LOOP,
        REDUNDANT_ENV_CLONE,
        UNNECESSARY_HOST_FUNCTION_CALL,
        SOROBAN_REDUNDANT_STORAGE_READ,
        STORAGE_WRITE_WITHOUT_READ,
        DISCARDED_STORAGE_READ,
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
        STORAGE_READ_MODIFY_WRITE,
    ]
}
