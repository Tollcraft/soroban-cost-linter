# `bytes_comparison_via_allocvec`

**Default Severity:** `warn`

**Target Resource:** [Memory — host and guest allocations](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags `==` and `!=` where both operands are `to_allocvec()` calls on a
`soroban_sdk::Bytes` value.

## Why is this bad?

{% hint style="danger" %}
`Bytes::to_allocvec()` copies the host value into a fresh guest `Vec<u8>`. Doing
it to both sides of a comparison pays two heap allocations and two full copies
for a question the host can answer with one direct `Bytes` comparison.
{% endhint %}

## Example

❌ Bad:

```rust
if left.to_allocvec() == right.to_allocvec() {
    // ...
}
```

✅ Good:

```rust
if left == right {
    // ...
}
```

## Suggested Fix

{% hint style="success" %}
Compare the `Bytes` values directly. Keep `to_allocvec()` for cases that
genuinely need owned guest bytes, such as passing them to a native API.
{% endhint %}

## What is not reported

- A direct `Bytes` comparison.
- A comparison with only one converted operand.
- Comparisons between native `Vec<u8>` values.
- Calls suppressed with `#[allow(bytes_comparison_via_allocvec)]`.
