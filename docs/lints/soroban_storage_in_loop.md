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

### Writes (`set`)

```rust
// ❌ Bad: one storage write per iteration
for item in items {
    env.storage().instance().set(&item, &1);
}
```

## Suggested Fix

{% hint style="success" %}
**For writes** (`env.storage().*.set(&k, &v)`), accumulate mutations in memory (using a `Vec` or `Map`) during the loop execution, then perform a single storage write outside of the loop.
{% endhint %}

### Reads (`get`, `has`)

```rust
// ❌ Bad: a loop-invariant read is repeated every iteration
const KEY: Symbol = symbol_short!("counter");
for _ in 0..10 {
    let _value = env.storage().instance().get(&KEY);
}
```

{% hint style="success" %}
**For reads** (`get` / `has`), the "buffer mutations" advice does not apply — reads cannot be accumulated. Hoist a loop-invariant read out of the loop (issue it once, bind to a local, reuse) where possible. A read keyed by the loop variable is typically unavoidable; in that case consider pre-fetching a `Vec`/`Map` of keys up front if the read itself dominates the cost.
{% endhint %}
