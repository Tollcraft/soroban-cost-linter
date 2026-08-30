# `bytes_slice_copy_in_loop`

**Default Severity:** `warn`

**Target Resource:** [Memory — host buffer allocation and copy cost](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects calls to sub-slice / copy-range methods (`slice`, `copy_from_slice`, `copy_to_slice`) on `soroban_sdk::Bytes` inside loop bodies (`for`, `while`, or `loop`).

## Why is this bad?

Taking a sub-slice of a Soroban `Bytes` allocates a new buffer and copies bytes through a metered host call. Doing this inside a loop that walks the buffer turns a linear traversal into a quadratic one: parsing a 1 KB payload four bytes at a time performs 256 slice operations, each copying an average of half the remaining buffer.

This is distinct from [`bytes_append_in_loop`](./bytes_append_in_loop.md), which catches quadratic *growth* (reallocating as the buffer gets larger). This lint catches quadratic *reading* (re-copying the remaining buffer on every iteration). They flag different code shapes and recommend different fixes.

## Example

```rust
use soroban_sdk::Bytes;

// ❌ Bad: each iteration slices and copies an average of half the
//          remaining buffer — O(n²) total copied bytes.
fn bad(payload: Bytes) {
    let mut i = 0u32;
    while i + 4 <= payload.len() {
        let _chunk = payload.slice(i, i + 4);
        i += 4;
    }
}
```

```rust
use soroban_sdk::Bytes;

// ✅ Good: slice once, then index into the result by position.
fn good(payload: Bytes) {
    let whole = payload.slice(0, payload.len());
    for i in (0..payload.len()).step_by(4) {
        let _byte = whole.get(i);
    }
}
```

## Cost impact

Each `Bytes::slice()` / `copy_from_slice()` / `copy_to_slice()` call allocates and copies a range of the buffer through a metered host call. When repeated per iteration of a loop that advances through the buffer, the per-iteration copy size stays large, so the total copied bytes grow with the square of the payload length — O(n²) memory traffic for what should be a linear scan.

## Small fixed-bound loops

A loop whose bound is small and known at compile time (e.g. walking a fixed 32-byte hash, or a constant `for i in 0..4`) only ever copies a tiny, constant amount of data. The quadratic cost only appears when the iteration count scales with the payload length. This is why the lint defaults to `warn` rather than `deny` — use `#[allow(bytes_slice_copy_in_loop)]` on the specific call site when the loop bound is small and bounded by contract invariants.

## Suggested Fix

- Prefer index-based access (`Bytes::get(i)`) for per-byte reads instead of slicing a sub-range each iteration.
- Take the full slice (or `copy_to_slice` into a native buffer) once, outside the loop, and iterate the result.
- Only slice once, not on every iteration.

## What is not reported

- Calls made outside a loop body.
- Calls on types that are not `soroban_sdk::Bytes` (e.g. `Vec`, `Map`, or native Rust `[u8]` / `Vec<u8>` slices).
- Methods other than `slice`, `copy_from_slice`, and `copy_to_slice`.
