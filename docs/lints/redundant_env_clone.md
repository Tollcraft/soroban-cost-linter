# `redundant_env_clone`

**Default Severity:** `warn`

**Target Resource:** [CPU — memory allocation, copy, and host object dispatch](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects unnecessary `.clone()` calls on the Soroban `Env` object, in both method
(`env.clone()`) and qualified/UFCS (`Env::clone(&env)`, `Clone::clone(&env)`) form.
The receiver may be a local binding, a struct field, or a value reached through a
reference target.

## Why is this bad?

{% hint style="danger" %}
The Soroban `Env` object is designed to be highly lightweight and is typically passed by value or reference. Cloning it forces `MemAlloc` and `MemCpy` operations followed by a `VisitObject` of the new handle — all unnecessary CPU cycles that the network charges for. See the [Cost Rationale — Metered Resources](../cost_rationale.md#1-cpu-instructions) for the cost types involved.
{% endhint %}

## Example

```rust
// ❌ Bad: Env is lightweight — no clone needed
let my_env = env.clone();
```

## Known False Positives (Not Flagged)

The lint does **not** fire when the clone is genuinely required for compilation:

1. **`&Env` receiver** — Cloning through a reference produces an owned `Env` from a borrowed one.
2. **Original binding reused after clone** — If the same `Env` binding is used again after the
   `.clone()` call, both the original and the clone are live.
3. **Temporary `Env`** — The result of a function call being cloned is not a reusable
   binding, so there is nothing for the clone to duplicate.

```rust
// ✅ Not flagged: &Env — clone produces owned Env from a reference
fn use_ref(env: &Env) {
    let owned = env.clone();
}

// ✅ Not flagged: env is used again after the clone
fn reused(env: Env) {
    let cloned = env.clone();
    some_function(env);  // original still live
}

// ✅ Not flagged: bound reference target — `r` is a `&Env`
fn bound_ref(env: Env) {
    let r = &env;
    let owned = r.clone();
}
```

## Suggested Fix

{% hint style="success" %}
Pass `env` directly by value or reference without calling `.clone()`.
{% endhint %}
