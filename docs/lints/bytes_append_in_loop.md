# `bytes_append_in_loop`

**Default Severity:** `warn`

**Target Resource:** [CPU — host function dispatch and serialization cost](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects calls to growth methods (`append`, `push_back`, `insert`, `extend_from_array`) on Soroban SDK container types (`Bytes`, `Vec`, `Map`) inside loop bodies (`for`, `while`, or `loop`).

## Why is this bad?

Each call to a growth method on a Soroban SDK container performs host-side work: the host must reallocate its internal buffer, copy existing elements, and serialize the new data. As the container grows, these operations become progressively more expensive because the buffer being reallocated and copied is larger each time.

Placing such calls inside a loop compounds this cost by the iteration count, turning an O(n) pattern into an O(n²) one for both CPU budget and memory allocation overhead.

## Example

```rust
use soroban_sdk::{Bytes, Vec};

// ❌ Bad: Host reallocates and copies an increasingly large buffer on
//          every iteration.
fn bad(bytes: &mut Bytes) {
    for _ in 0..10 {
        bytes.append(&other);
    }
}
```

```rust
use soroban_sdk::{Bytes, Vec};

// ✅ Good: Accumulate in a native Vec, then convert to the SDK container
//          once after the loop.
fn good(items: &[u8]) -> Bytes {
    let mut buf = Vec::new();
    for item in items {
        buf.push(*item);
    }
    Bytes::from(buf)
}
```

## Small fixed-bound loops

A loop with a small, fixed number of iterations (e.g. 2–3) is usually inexpensive in absolute terms. This is why the lint defaults to `warn` rather than `deny` — use `#[allow(bytes_append_in_loop)]` on the specific call site when the iteration count is small and bounded.

## Suggested Fix

- Accumulate values in native Rust collections (`Vec`, `HashMap`, etc.) during the loop.
- Convert to the Soroban SDK container once after the loop using `From`/`Into` or equivalent.
- Pre-size your native collection with `Vec::with_capacity` when the final size is known.

## What is not reported

- Calls made outside a loop body.
- Calls on types that are not `soroban_sdk::Bytes`, `soroban_sdk::Vec`, or `soroban_sdk::Map`.
- Methods other than `append`, `push_back`, `insert`, and `extend_from_array`.
