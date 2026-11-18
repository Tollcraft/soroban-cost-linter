# `linear_scan_in_loop`

**Default Severity:** `warn`

**Target Resource:** [CPU — collection scanning operations](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects linear-time (O(n)) scanning operations on Soroban collection types
(`soroban_sdk::vec::Vec` and `soroban_sdk::map::Map`) that are called inside a
loop body when the scanned collection does not change between iterations.

The following methods are flagged when called on a Soroban collection:

| Method       | Type        | Cost      |
| ------------ | ----------- | --------- |
| `.contains`  | `Vec`       | O(n) scan |
| `.position`  | `Vec`       | O(n) scan |
| `.find`      | `Vec`       | O(n) scan |

## Why is this bad?

Placing a linear scan inside a loop produces O(n²) total cost: every iteration
rescans the entire collection from scratch. For a collection with 100 items
iterated 100 times, that is 10 000 element comparisons instead of a constant-time
lookup.

## Example

```rust
// ❌ Bad: O(n²) — rescans items on every iteration
for i in 0..n {
    if items.contains(&target) {
        // ...
    }
}
```

```rust
// ✅ Good: build a Map lookup once, then check in O(1) per iteration
let lookup: Map<i32, bool> = build_lookup(&items);
for i in 0..n {
    if lookup.get(&target).unwrap_or(false) {
        // ...
    }
}
```

## Suggested Fix

Build an index (`Map` or `Set`) from the scanned collection *before* the loop
starts, then use O(1) key lookups inside the loop body instead of O(n) scans.

## Known false positives

A call that reads the loop variable (or any value that changes across iterations)
is left alone because the scan does genuinely different work each time. The lint
may still report a call if mutation analysis cannot prove loop-dependence, but
this is rare in practice.
