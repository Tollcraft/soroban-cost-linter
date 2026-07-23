# `unnecessary_host_function_call`

**Default Severity:** `warn`

**Target Resource:** [CPU — host function dispatch and execution](../cost_rationale.md#per-lint-resource-summary)

## What it does

Identifies redundant calls to host functions, such as fetching the ledger sequence or timestamp, inside loop bodies.

## Why is this bad?

{% hint style="danger" %}
Calling a host function crosses the Wasm-host boundary, which incurs `DispatchHostFunction` overhead plus whatever work the function performs. Repeating this unnecessarily inside a loop adds up to **significant CPU waste**, especially when the result is constant across iterations. See the [Cost Rationale — What Dominates](../cost_rationale.md#what-dominates) for the relative cost hierarchy.
{% endhint %}

## Example

```rust
// ❌ Bad: same host call repeated every iteration
for item in items {
    let current_seq = env.ledger().sequence();
}
```

## Suggested Fix

{% hint style="success" %}
Call the host function once before the loop, store the result in a local variable, and reference that variable inside the loop.
{% endhint %}
