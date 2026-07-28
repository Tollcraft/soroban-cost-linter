# `excessive_vec_capacity`

**Default Severity:** `warn`

**Target Resource:** [Memory — guest linear memory (RAM, hard-capped)](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags `Vec::with_capacity(N)`, `.reserve(N)`, and `.reserve_exact(N)` calls on
a Rust `Vec` where `N` is a hard-coded integer literal above `1024`.

## Why is this bad?

{% hint style="danger" %}
Soroban contracts run inside a guest Wasm instance with a small, hard-capped
linear memory budget (see [Cost Rationale — Memory](../cost_rationale.md#2-memory-ram)).
Unlike a long-running server process, where reserving a large `Vec` up front
is a cheap, common optimization, a Soroban guest pays for that allocation the
moment it is made — regardless of whether the capacity is ever filled. A
large, hard-coded capacity that isn't tied to a known, bounded input size is
a structural sign of over-allocation: either the real bound is much smaller
and the literal is a guess, or the vector is filled incrementally and never
needed the reservation at all.
{% endhint %}

## Example

```rust
// ❌ Bad: a hard-coded, oversized reservation that isn't tied to any known bound
let mut items = Vec::with_capacity(10_000);
```

```rust
// ✅ Good: capacity matches a known, bounded input size
let mut items = Vec::with_capacity(input.len());
```

```rust
// ✅ Good: grow incrementally when there's no known upper bound
let mut items = Vec::new();
for entry in input {
    items.push(entry);
}
```

## Suggested Fix

{% hint style="success" %}
Size the reservation to a known, bounded input (`input.len()`, a fixed
protocol constant, etc.), or drop the reservation and let the vector grow
incrementally if no such bound exists.
{% endhint %}

## What is not reported

- `Vec::with_capacity(N)` / `.reserve(N)` / `.reserve_exact(N)` where `N` is
  `1024` or below, or is not a literal (e.g. `Vec::with_capacity(input.len())`) —
  the lint only flags a large, hard-coded constant, since a non-literal
  argument is already tied to a runtime-computed bound.
- Capacity calls on Soroban SDK containers (`soroban_sdk::Vec`, `Map`,
  `Bytes`): none of them expose a `with_capacity`/`reserve` API today.
- Calls suppressed with `#[allow(excessive_vec_capacity)]`.

## Deliberately not covered

- **Data-flow tracking of subsequent fills** — the lint does not check how
  many elements are actually pushed after the reservation; it flags the
  reservation itself based purely on the literal's size. A large capacity
  that genuinely gets filled by a bounded, statically-known loop is still
  flagged; the fix in that case is to compute the capacity from that same
  bound (e.g. `Vec::with_capacity(n)` where `n` is the loop bound) rather
  than a separate hard-coded literal.
- **`HashMap`/`BTreeMap`/`VecDeque` capacity reservations** — the same
  over-allocation concern applies to these std collections, but is left as a
  follow-up once this pattern is validated for `Vec`.
