# `unnecessary_string_to_bytes`

**Default Severity:** `warn`

## What it does

Detects unnecessary `.to_bytes()` calls on the Soroban `String` object.

## Why is this bad?

{% hint style="danger" %}
Converting a `soroban_sdk::String` to `Bytes` via `.to_bytes()` creates a new host-backed object, which incurs **unnecessary CPU and memory costs**. In many cases, `String` can be used directly where `Bytes` is accepted, such as in storage operations.
{% endhint %}

## Example

```rust
// ❌ Bad: unnecessary conversion from String to Bytes
let key = String::from_str(&env, "foo");
let key_bytes = key.to_bytes();
env.storage().instance().set(&key_bytes, &value);
```

## Suggested Fix

{% hint style="success" %}
Use the `String` directly where `Bytes` is accepted, or construct `Bytes` directly using `Bytes::from_slice()` instead of converting from a `String`.
{% endhint %}

```rust
// ✅ Good: use String directly
let key = String::from_str(&env, "foo");
env.storage().instance().set(&key, &value);

// ✅ Also good: construct Bytes directly
let key = Bytes::from_slice(&env, b"foo");
env.storage().instance().set(&key, &value);
```
