# `soroban_storage_in_loop`

**Default Severity:** `warn`

**Target Resource:** [Storage — ledger entry accesses and ledger I/O bytes](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects storage operations (reads or writes) that are executed inside loop bodies (`for`, `while`, or `loop`).

## Why is this bad?

{% hint style="danger" %}
Storage operations are the **single most expensive resource** Soroban charges for. Each storage write consumes a ledger entry write access, I/O bytes, serialization cost, and (for new entries) space rent. Placing them inside a loop multiplies every dimension by the iteration count. See the [Cost Rationale — Storage](../cost_rationale.md#3-storage-ledger-entry-accesses-and-ledger-io) for details.
{% endhint %}

## Example

```rust
// ❌ Bad: one storage write per iteration
for item in items {
    env.storage().instance().set(&item, &1);
}
```

## Suggested Fix

{% hint style="success" %}
Accumulate mutations in memory (using a `Vec` or `Map`) during the loop execution, and perform a single storage read/write operation outside of the loop.
{% endhint %}
