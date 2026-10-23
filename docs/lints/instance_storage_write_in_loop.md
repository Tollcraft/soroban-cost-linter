# `instance_storage_write_in_loop`

**Default Severity:** `warn`

**Target Resource:** [Storage — ledger entry accesses and ledger I/O bytes](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects `set`, `put` and `update` calls on the **instance** storage tier that
run inside a loop or inside a closure the loop drives (`for_each`, `map`,
`fold`). Reads (`get`, `has`) and writes to the `Persistent` and `Temporary`
tiers are left to their own lints.

## Why is this bad?

{% hint style="danger" %}
Instance storage travels with the contract instance: it is loaded on every
invocation and every write is paid for again by whoever reads the instance
next. Writing it once per iteration multiplies the ledger write cost by the
trip count while only the last write is visible afterwards. See the
[Cost Rationale — Storage](../cost_rationale.md#3-storage-ledger-entry-accesses-and-ledger-io)
for details.
{% endhint %}

## Example

```rust
// ❌ Bad: one instance write per iteration
for i in 0..10 {
    env.storage().instance().set(&i, &i);
}

// ✅ Good: accumulate in memory, write the instance entry once
let mut total = 0;
for i in 0..10 {
    total += i;
}
env.storage().instance().set(&"total", &total);
```

## Suggested Fix

Accumulate the mutations in memory during the loop and write the instance
entry once after it. If every iteration genuinely needs a distinct instance
entry, the data is not instance-sized — consider persistent storage or a
batched `Map` under a single key.

## Relationship to `soroban_storage_in_loop`

`soroban_storage_in_loop` denies storage operations in loops generally and is
registered first, so with both lints at their defaults the general diagnostic
is emitted first. This lint adds the instance-tier specific message and the
closure shapes; fixtures that want each diagnostic attributable to this lint
allow `soroban_storage_in_loop` in their header.
