# `unbounded_input_loop`

**Default Severity:** `warn`

**Target Resource:** [Storage — ledger entry accesses and CPU](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags loops whose iteration count is derived from an untrusted function parameter and whose body performs a storage write. Such loops can be abused to exhaust the contract's CPU and memory budget.

## Why is this bad?

{% hint style="danger" %}
Loops that iterate an unbounded number of times based on untrusted input present a denial-of-service vector. An attacker can craft input that forces the loop to run thousands or millions of iterations, each performing a metered storage write, quickly exhausting the contract's resource budget. See the [Cost Rationale — Storage](../cost_rationale.md#3-storage-ledger-entry-accesses-and-ledger-io) for details.
{% endhint %}

## Example

```rust
// ❌ Bad: loop bound comes from untrusted input
pub fn process(env: Env, items: Vec<u32>) {
    for item in items.iter() {
        env.storage().instance().set(&item, &1u32);
    }
}
```

```rust
// ✅ Good: bound is clamped to a safe constant
pub fn process(env: Env, items: Vec<u32>) {
    let max_items = 100;
    let count = items.len().min(max_items);
    for i in 0..count {
        env.storage().instance().set(&i, &1u32);
    }
}
```

## Cost impact

An unbound loop whose iteration count is attacker-controlled can execute as many storage writes as the caller's budget allows, potentially draining the contract's available CPU and memory resources.

## Suggested Fix

{% hint style="success" %}
Clamp the loop bound with `.min(CONST)` or validate the input size before using it as a loop bound. Alternatively, process items in batches with a bounded number per invocation.
{% endhint %}
