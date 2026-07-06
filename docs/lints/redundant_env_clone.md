# `redundant_env_clone`

**Default Severity:** `warn`

## What it does

Detects unnecessary `.clone()` calls on the Soroban `Env` object.

## Why is this bad?

{% hint style="danger" %}
The Soroban `Env` object is designed to be highly lightweight and is typically passed by value or reference. Cloning it incurs **unnecessary CPU cycles**.
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
