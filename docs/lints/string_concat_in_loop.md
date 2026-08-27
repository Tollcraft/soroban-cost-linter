# `string_concat_in_loop`

**Default Severity:** `warn`

**Target Resource:** [Memory — host buffer allocation and copy cost](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects concatenation of a Soroban `soroban_sdk::String` inside loop bodies (`for`, `while`, or `loop`). It flags two shapes:

- `append` calls on a `String` receiver (`result = result.append(&piece)`).
- `String + String` (`Add`) expressions (`result = result + piece`).

## Why is this bad?

`soroban_sdk::String` is a host-side object. Each concatenation (`append` or `+`) allocates a fresh host buffer and copies **everything accumulated so far** into it. As the string grows, every iteration copies a larger and larger prefix, so building a string from `n` pieces inside a loop performs O(n²) byte copies for both CPU budget and memory allocation overhead — the exact quadratic growth that [`bytes_append_in_loop`](bytes_append_in_loop.md) already catches for `Bytes`, `Vec`, and `Map`.

Strings show up in contracts for metadata, symbol names, and error context, usually assembled in precisely the loop shape that triggers this lint.

## Example

```rust
use soroban_sdk::{String, Env};

// ❌ Bad: the host reallocates and copies an increasingly large buffer on
//          every iteration.
fn bad(env: &Env, pieces: &[&str]) {
    let mut result = String::from_str(env, "");
    for p in pieces {
        let piece = String::from_str(env, p);
        result = result.append(&piece); // flagged
    }
}
```

```rust
use soroban_sdk::{String, Env};

// ✅ Good: accumulate the pieces in a native `Vec` inside the loop, then
//          construct the `String` a single time afterwards.
fn good(env: &Env, pieces: &[&str]) -> String {
    let mut buf: Vec<String> = Vec::new();
    for p in pieces {
        buf.push(String::from_str(env, p));
    }
    // Build once — only one host-side allocation/copy.
    String::from_str(env, "joined")
}
```

## Cost impact

Each `String::append()` or `String + String` call performs a host-side allocation and copies the entire existing string contents. Because the copied prefix grows with every iteration, the pattern is O(n²) in the total number of characters produced.

Measured with `Env::default()` in the [`cost_benchmarks`](https://github.com/Tollcraft/soroban-cost-linter/tree/main/cost_benchmarks) crate (`cargo test -- --nocapture`):

| Pattern | Iterations | CPU instructions (delta) | Memory bytes (delta) |
| --- | --- | --- | --- |
| `String::append()` in loop (bad) | 100 | *run `cargo test -- --nocapture` in `cost_benchmarks/`* | *run `cargo test -- --nocapture` in `cost_benchmarks/`* |
| Native `Vec<String>` + single `String` construction (good) | 100 | *run `cargo test -- --nocapture` in `cost_benchmarks/`* | *run `cargo test -- --nocapture` in `cost_benchmarks/`* |

{% hint style="warning" %}
The per-iteration cost **grows with the accumulated string length** — each successive `append` is more expensive than the last. Larger payloads amplify the copy overhead.
{% endhint %}

### How to reproduce

```bash
cd cost_benchmarks
cargo test bench_string_concat_in_loop_vs_batch -- --nocapture
```

## Why a separate lint rather than extending `bytes_append_in_loop`?

`bytes_append_in_loop` targets growth *methods* (`append`, `push_back`, `insert`, `extend_from_array`) on `Bytes`, `Vec`, and `Map`, and it keys off those specific method names. `String` concatenation is a different surface:

- `append` on `String` returns a **new** `String` (it is not an `&mut self` in-place growth like `Bytes::append`/`Vec::push_back`), and `String` also supports the `Add` operator, which has no analogue in the container types.
- The set of types is disjoint: extending the container lint's type table with `soroban_sdk::String` would either (a) mis-fire on `String`'s `append` returning a value rather than mutating, or (b) require method-name special-casing that muddies the container lint's "in-place growth" semantics.

Keeping `string_concat_in_loop` separate preserves the clear, single-responsibility messages of both lints (collect-then-join vs. accumulate-then-batch) and avoids coupling two distinct cost stories. The trade-off is a little duplicated loop-detection scaffolding, which is acceptable given how small and stable it is.

## Small fixed-bound loops

A loop with a small, fixed number of iterations (e.g. 2–3) is usually inexpensive in absolute terms. This lint does **not** attempt to prove the bound of a loop — it mirrors [`bytes_append_in_loop`](bytes_append_in_loop.md)'s deliberately conservative posture and fires on **any** syntactic loop. This is a **known false positive**: a tiny, provably-bounded loop still triggers the warning.

- **Handling:** suppress the specific call site with `#[allow(string_concat_in_loop)]`, or accumulate the (small number of) pieces in a native collection and construct the `String` once anyway.
- **Rationale for not adding bound detection:** proving loop bounds statically is runtime-dependent and would inflate the false-negative rate for the genuinely expensive unbounded cases this lint exists to catch. Consistency with the sibling `bytes_append_in_loop` lint (which also omits bound analysis) keeps the codebase's loop-lint story uniform.

## Suggested Fix

- Accumulate the pieces in a native Rust collection (`Vec<String>`, `Vec<&str>`, or `Vec<Bytes>`) during the loop.
- Construct the `String` **once** after the loop (e.g. `String::from_str` / `Bytes` join).
- Pre-size the native collection with `Vec::with_capacity` when the final size is known.

## What is not reported

- Calls made outside a loop body.
- Concatenation on types other than `soroban_sdk::String` (e.g. `std::string::String` in host-side code, or `soroban_sdk::Bytes`).
- Methods other than `append`, and operator forms other than `String + String`.
