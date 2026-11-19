---
description: Large value decoded for one field — decoding a stored struct to read only a single field
---

# `large_value_decoded_for_one_field`

| Default Severity | Category |
| ---------------- | -------- |
| `warn`           | Storage  |

## What it does

Flags deserialization of a multi-field composite struct from storage when only a single field is subsequently accessed in the enclosing scope.

## Why is this bad

Deserializing an entire multi-field struct from storage when only one field is needed decodes all unused fields into memory, wasting host CPU cycles and memory allocations. Storing frequently accessed scalar fields under separate granular storage keys allows reading only the required bytes.

## Example

**Bad** — deserializing a multi-field struct to access only one field:

```rust
// ❌ Triggers: large_value_decoded_for_one_field
fn get_admin_fee(env: Env, key: &Symbol) -> u32 {
    let config: Config = env.storage().instance().get(key).unwrap();
    config.fee
}
```

**Good** — store frequently read fields under dedicated granular storage keys or read multiple fields:

```rust
// ✅ Fixed: read dedicated granular storage key
fn get_admin_fee(env: Env, fee_key: &Symbol) -> u32 {
    env.storage().instance().get(fee_key).unwrap()
}
```

## Fix

Store scalar or frequently read fields under separate granular keys, or avoid decoding the entire struct when only one field is needed.
