# `nested_loop_storage_access`

**Default Severity:** `deny`

**Target Resource:** [Storage — ledger entry accesses and ledger I/O bytes](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects any storage operation (`get`, `has`, `set`, `update`, `remove`,
`extend_ttl`, …) that runs **two or more loops deep**.

A closure body does not add a nesting level: a `for_each` inside one loop is
still depth one, because the enclosing loop determines how often it runs. A
`fn` defined inside a loop resets the counter and is analysed on its own.

## Why is this bad?

{% hint style="danger" %}
One loop costs O(n) host calls for a storage operation. Two loops cost
O(n·m) — the product of both trip counts — which is qualitatively worse than
the linear case and almost never intentional. This is why the lint is declared
`Deny`: the finding is an error, not a warning. See the
[Cost Rationale — Storage](../cost_rationale.md#3-storage-ledger-entry-accesses-and-ledger-io)
for the per-access charges.
{% endhint %}

## Example

```rust
// ❌ Bad: every (i, j) pair pays a storage write
for i in 0..n {
    for j in 0..m {
        env.storage().instance().set(&(i, j), &1);
    }
}

// ✅ Good: accumulate in memory, write once after both loops
let mut map = Map::new(&env);
for i in 0..n {
    for j in 0..m {
        map.set((i, j), 1);
    }
}
env.storage().persistent().set(&"pairs", &map);
```

## Suggested Fix

Hoist the storage access out of at least one loop, or accumulate the mutations
in an in-memory `Map`/`Vec` and perform a single storage write once both loops
have finished. When the multiplicative cost is intentional, suppress the lint
with `#[allow(nested_loop_storage_access)]` on the enclosing item.
