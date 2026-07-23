# `redundant_env_clone`

**Default Severity:** `warn`

**Target Resource:** [CPU — memory allocation, copy, and host object dispatch](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects unnecessary `.clone()` calls on the Soroban `Env` object.

## Why is this bad?

{% hint style="danger" %}
The Soroban `Env` object is designed to be highly lightweight and is typically passed by value or reference. Cloning it forces `MemAlloc` and `MemCpy` operations followed by a `VisitObject` of the new handle — all unnecessary CPU cycles that the network charges for. See the [Cost Rationale — Metered Resources](../cost_rationale.md#1-cpu-instructions) for the cost types involved.
{% endhint %}

## Example

```rust
// ❌ Bad: Env is lightweight — no clone needed
let my_env = env.clone();
```

## Suggested Fix

{% hint style="success" %}
Pass `env` directly by value or reference without calling `.clone()`.
{% endhint %}
