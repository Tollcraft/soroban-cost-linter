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

## Cost impact

Each `Bytes::append()`, `Vec::push_back()`, or `Map::insert()` call performs host-side work: the host reallocates its internal buffer, copies existing elements, and serializes the new data. As the container grows, each call becomes **more expensive** because the buffer being reallocated is larger — turning the pattern into O(n²) for both CPU and memory.

Measured with `Env::default()` in the [`cost_benchmarks`](https://github.com/Tollcraft/soroban-cost-linter/tree/main/cost_benchmarks) crate (`cargo test -- --nocapture`):

| Pattern | Iterations | CPU instructions (delta) | Memory bytes (delta) |
| --- | --- | --- | --- |
| `Bytes::append()` in loop (bad) | 100 | *run `cargo test -- --nocapture` in `cost_benchmarks/`* | *run `cargo test -- --nocapture` in `cost_benchmarks/`* |
| Native `Vec` + `Bytes::from_slice()` once (good) | 100 | *run `cargo test -- --nocapture` in `cost_benchmarks/`* | *run `cargo test -- --nocapture` in `cost_benchmarks/`* |

{% hint style="warning" %}
The per-iteration cost **grows with the container size** — each successive `append` is more expensive than the last. With 100 items the last few appends are dramatically costlier than the first few. Larger payloads amplify the serialization overhead.
{% endhint %}

### How to reproduce

```bash
cd cost_benchmarks
cargo test bench_bytes_append_in_loop_vs_batch -- --nocapture
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
