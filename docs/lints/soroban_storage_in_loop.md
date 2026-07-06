# `soroban_storage_in_loop`

**Default Severity:** `warn`

## What it does

Detects storage operations (reads or writes) that are executed inside loop bodies (`for`, `while`, or `loop`).

## Why is this bad?

{% hint style="danger" %}
Storage operations in Soroban are the **most expensive resource**. Placing them inside a loop wastes CPU instructions and network read/write throughput, drastically increasing your transaction fees.
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
