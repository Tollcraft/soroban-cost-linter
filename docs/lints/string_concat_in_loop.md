# `string_concat_in_loop`

**Default Severity:** `warn`

**Target Resource:** [Memory — host buffer allocation and copy cost](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects calls to `soroban_sdk::String::from_bytes` inside loop bodies (`for`, `while`, or `loop`) when the converted byte buffer is a local declared outside the loop and grows within the loop. Soroban SDK 28 does not expose `String::append` or implement `Add`; repeatedly reconstructing a string from a growing byte slice is a supported conversion shape that can recopy the accumulated buffer. Independent per-iteration slices and buffers cleared or reassigned before being refilled are not reported.

## Why is this bad?

`soroban_sdk::String` is a host-side object. Constructing it from bytes copies and converts the supplied slice. If that slice is a growing buffer, repeatedly calling `from_bytes` in a loop recopies the accumulated prefix, producing quadratic copying as the buffer grows.

Strings show up in contracts for metadata, symbol names, and error context, usually assembled in precisely the loop shape that triggers this lint.

## Example

```rust
use soroban_sdk::{Env, String};

// ❌ Bad: repeatedly convert the growing byte buffer into a host String.
fn bad(env: &Env, pieces: &[u8]) {
    let mut bytes = std::vec::Vec::new();
    for piece in pieces {
        bytes.push(*piece);
        let _result = String::from_bytes(env, &bytes); // flagged
    }
}
```

```rust
use soroban_sdk::{Env, String};

// ✅ Good: finish the native byte buffer first, then convert once.
fn good(env: &Env, pieces: &[u8]) -> String {
    let mut bytes = std::vec::Vec::new();
    for piece in pieces {
        bytes.push(*piece);
    }
    String::from_bytes(env, &bytes)
}
```

## Cost impact

Each `String::from_bytes` call constructs a host-backed string from the full byte slice. Repeating that conversion on an ever-growing slice can copy O(n²) bytes overall. This lint targets the structural pattern; the repository does not currently include a dedicated benchmark for this SDK 28 API.

## Why a separate lint rather than extending `bytes_append_in_loop`?

`bytes_append_in_loop` covers growth of Soroban byte containers. This lint covers the String-construction side by reporting `String::from_bytes` calls in loops; `Bytes::append` remains the container lint's responsibility.

## Small fixed-bound loops

A loop with a small, fixed number of iterations (e.g. 2–3) is usually inexpensive in absolute terms. This lint does **not** attempt to prove the bound of a loop — it mirrors [`bytes_append_in_loop`](bytes_append_in_loop.md)'s deliberately conservative posture and fires on **any** syntactic loop. This is a **known false positive**: a tiny, provably-bounded loop still triggers the warning.

- **Handling:** suppress the specific call site with `#[allow(string_concat_in_loop)]`, or construct the `String` once after accumulating bytes.
- **Rationale for not adding bound detection:** proving loop bounds statically is runtime-dependent and would inflate the false-negative rate for the genuinely expensive unbounded cases this lint exists to catch. Consistency with the sibling `bytes_append_in_loop` lint (which also omits bound analysis) keeps the codebase's loop-lint story uniform.

## Suggested Fix

- Accumulate the bytes in a native Rust collection during the loop.
- Construct the `String` **once** after the loop with `String::from_bytes`.
- Pre-size the native collection with `Vec::with_capacity` when the final size is known.

## What is not reported

- Calls made outside a loop body.
- `String::from_bytes` outside a loop.
- Other constructors such as `String::from_str`.
