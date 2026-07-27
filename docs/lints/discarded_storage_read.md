# `discarded_storage_read`

**Default Severity:** `warn`

## What it does

Detects `get` calls on instance, persistent, or temporary storage whose result is discarded (statement expression) or bound to a wildcard (`_`).

## Why is this bad?

{% hint style="danger" }
Storage reads in Soroban are metered host calls that cost CPU instructions and bandwidth. A discarded `get` pays the full read cost without using the result, wasting transaction fees for no benefit.
{% endhint %}

This lint catches two common waste patterns:

1. **Statement expression** — `env.storage().instance().get(&key);` where the return value is never captured.
2. **Wildcard binding** — `let _ = env.storage().instance().get(&key);` where the result is explicitly ignored.

{% hint style="info" %}
This lint does **not** fire on `has()` checks or reads whose result flows into a subsequent expression (e.g. `if let Some(v) = ...`, `.is_some()`). Those are legitimate uses of a storage read.
{% endhint %}

### vs. rustc `unused_variables`

rustc already warns when a `let` binding is unused. This lint adds signal beyond that by:

- Catching **statement expressions** (`get(&key);`) where no binding exists at all — rustc does not emit `unused_variables` for these.
- Catching **wildcard bindings** (`let _ = get(&key)`) which rustc treats as intentionally ignored.
- Framing the cost in **Soroban-specific terms** ("metered host read wasted") and suggesting `has()` as a cheaper alternative.

## Example

```rust
// ❌ Bad: result of metered read is discarded
env.storage().instance().get::<i32, i32>(&key);

// ❌ Bad: result explicitly ignored — still paid the read cost
let _ = env.storage().persistent().get::<i32, i32>(&key);
```

## Suggested Fix

```rust
// ✅ Good: use has() if you only need to check existence
if env.storage().instance().has(&key) {
    // ...
}

// ✅ Good: bind and use the result
if let Some(val) = env.storage().instance().get::<i32, i32>(&key) {
    // use val
}
```

## Known False Positives

None currently identified.
