# `persistent_storage_for_ephemeral_data`

**Default Severity:** `warn`

**Target Resource:** [Storage — ledger entry accesses, ledger I/O bytes, and space rent](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects a `persistent().set(&key, ..)` whose key is removed again on **every
control-flow path** through the function — an unconditional `remove`, or an
`if`/`else` whose two arms both remove the key.

The lint stays silent when a path can leave the function before the removal
(an early `return`, a `?`), when only one branch removes the key, when the
removal happens inside a loop, and when the written and removed keys differ.

## Why is this bad?

{% hint style="danger" %}
A persistent entry is paid for by the archival ledger write that keeps it
alive and by space rent for as long as it exists. If the value is written and
torn down before the call returns, nothing ever reads it across invocations,
and the persistent tier buys nothing over temporary storage. See the
[Cost Rationale — Storage](../cost_rationale.md#3-storage-ledger-entry-accesses-and-ledger-io).
{% endhint %}

## Example

```rust
// ❌ Bad: persistent write, removed before the call returns
env.storage().persistent().set(&key, &value);
env.storage().persistent().remove(&key);

// ✅ Good: temporary storage — the value never outlives the call
env.storage().temporary().set(&key, &value);
env.storage().temporary().remove(&key);
```

## Suggested Fix

Use the `Temporary` tier for scratch values that never outlive the
invocation, or keep the value in memory if only this call reads it.
