---
description: Storage write without corresponding read — flag unnecessary writes that waste Soroban storage fees
sidebar_position: 4
---

# `storage_write_without_read`

| Default Severity | Category     |
| ---------------- | ------------ |
| `warn`           | StorageOperations |

## What It Catches

When your Soroban contract writes to storage (`.set()`) on a storage object but never reads that value back with `.get()` or checks existence with `.has()` using the same key — you're paying storage write fees for data that's never used.

## Example

**Bad** — writing a value that is never read wastes storage write fees:

```rust
// ❌ Triggers: storage_write_without_read
fn update(env: Env, key: &str) {
    env.storage().instance().set(key, &1);
    // The value at `key` is never read back
}
```

**Good** — read the value back if you intend to use it:

```rust
// ✅ Fixed: read before write
fn update(env: Env, key: &str) {
    let _existing: Option<i32> = env.storage().instance().get(key);
    env.storage().instance().set(key, &1);
}
```

## Fix

Either add a corresponding `.get()` or `.has()` call on the same storage object with the same key before the write, or remove the unnecessary write altogether.