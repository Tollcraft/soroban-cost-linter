# `vec_index_in_loop`

**Default Severity:** `warn`

**Target Resource:** [CPU — collection indexing operations](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects loops that iterate over a range and index a `soroban_sdk::Vec` by the loop variable (e.g., using `v.get(i)` or `v.get_unchecked(i)`).

## Why is this bad?

Indexing a Soroban `Vec` is a host call that performs a bounds check and a conversion on every access. In contrast, walking the collection with `for item in v.iter()` pays a single traversal cost and avoids repeated bounds checks and conversion overhead.

## Example

```rust
// ❌ Bad: repeated host calls and bounds checks per element
for i in 0..v.len() {
    let item = v.get(i);
    // ...
}
```

```rust
// ✅ Good: single traversal cost via iteration
for item in v.iter() {
    // ...
}
```

## Suggested Fix

Use `.iter()` to iterate over the collection directly instead of indexing it by the loop counter.

## Known false positives

- Loops that mutate the collection inside the loop body are ignored because converting them to iteration would fail to compile due to borrow checker rules.
