# `host_container_clone_in_loop`

**Default Severity:** `warn`

**Target Resource:** [Memory / RAM — host container allocations and deep copies](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags `.clone()` calls on Soroban SDK host containers (`soroban_sdk::Vec`, `Map`, `Bytes`, and `String`) made inside a loop body (`for`, `while`, or `loop`).

## Why is this bad?

{% hint style="danger" %}
Cloning a `soroban_sdk::Vec`, `Map`, `Bytes`, or `String` deep-copies the host object on every iteration. The host serializes and duplicates the whole container payload $N$ times for a value that is usually read-only.
{% endhint %}

## Example

```rust
// ❌ Bad: cloning an SDK container inside a loop
for item in items.iter() {
    let copy = container.clone();
    process(item, &copy);
}
```

```rust
// ✅ Good: borrow the container or move the clone before the loop
for item in items.iter() {
    process(item, &container);
}
```

## Suggested Fix

{% hint style="success" %}
Borrow the container or move the clone before the loop.
{% endhint %}

## Deliberate difference from `redundant_env_clone`

Unlike `redundant_env_clone`, which inspects cheap `Env` handles and skips reporting when the receiver is a reference or when the local binding is still used later, cloning a host container always performs a real copy regardless of whether the original is live or whether the receiver is borrowed.

## What is not reported

- `.clone()` calls on SDK containers outside a loop body.
- Hoisting `.clone()` before the loop and referencing the clone inside the loop.
- `.clone()` on native Rust types such as `std::vec::Vec`.
- Calls suppressed with `#[allow(host_container_clone_in_loop)]`.
