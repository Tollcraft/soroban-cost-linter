# `soroban_redundant_storage_read`

**Default Severity:** `warn`

## What it does

Detects when the same storage key is read multiple times in sequence without an intervening write. This catches redundant `get` and `has` calls on `Instance`, `Persistent`, and `Temporary` storage.

## Why is this bad?

{% hint style="danger" %}
Storage reads in Soroban consume read-throughput bandwidth. Reading the same key multiple times without modification wastes this resource and increases transaction fees.
{% endhint %}

## Example

```rust
// ❌ Bad: two sequential reads of the same key
let a: Option<i32> = env.storage().instance().get(&key);
let b: Option<i32> = env.storage().instance().get(&key); // redundant!
```

## Suggested Fix

{% hint style="success" %}
Store the value from the first read and reuse it instead of reading the same key again.
{% endhint %}

```rust
// ✅ Good: read once, reuse the value
let a: Option<i32> = env.storage().instance().get(&key);
let b: Option<i32> = a; // reuse the first read
```

## Scope

- Fires on sequential `get` and `has` calls on `Instance`, `Persistent`, and `Temporary` storage types.
- A `set` (write) resets the tracking, so reads before and after a write are not flagged.
- Reads of different keys (even on the same storage type) are not flagged.
- Only tracks reads at the top level of statements within the same block. Nested block tracking is not yet supported.
