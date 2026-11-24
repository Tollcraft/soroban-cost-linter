# `repeated_instance_write_same_key`

**Default Severity:** `warn`

**Target Resource:** [Storage — ledger entry accesses and ledger I/O bytes](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags multiple `set` calls on the `Instance` storage tier using the same key within a single function body without an intervening read.

## Why is this bad?

{% hint style="danger" %}
Each `env.storage().instance().set(&key, ..)` serializes and rewrites the entire contract instance storage map and incurs its own metered ledger write. Writing twice to the same key in one function execution pays twice for storage writes while only the final value survives.
{% endhint %}

## Example

```rust
// ❌ Bad: writing to the same instance key twice in one function
fn update_state(env: &Env, key: &Symbol) {
    env.storage().instance().set(key, &1);
    // ... logic ...
    env.storage().instance().set(key, &2);
}
```

```rust
// ✅ Good: compute the final value first and write it once
fn update_state(env: &Env, key: &Symbol) {
    let final_val = 2;
    env.storage().instance().set(key, &final_val);
}
```

## Suggested Fix

{% hint style="success" %}
Compute the final value once and write it after the branch or loop that decides it.
{% endhint %}

## What is not reported

- Writes to different keys.
- Writes across different function bodies.
- A read-modify-write pattern where `get` intervenes between writes.
- Writes to other storage tiers (`Persistent`, `Temporary`).
- Functions suppressed with `#[allow(repeated_instance_write_same_key)]`.
