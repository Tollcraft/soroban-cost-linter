---
description: Storage write without corresponding read — flag unnecessary writes that waste Soroban storage fees
sidebar_position: 20
---

# `storage_write_without_read`

| Default Severity | Category     |
| ---------------- | ------------ |
| `warn`           | StorageOperations |

## What it does

Flags storage writes (`.set()`) on a storage object when the same key is never read back with `.get()` or checked with `.has()` — indicating a write whose value is never used.

## Why is this bad

Performing a storage write without reading the value first wastes Soroban storage write fees for data that's never used. Each unnecessary write consumes CPU and memory budget, adding cost without benefit.

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

## Suggested Fix

Either add a corresponding `.get()` or `.has()` call on the same storage object with the same key before the write, or remove the unnecessary write altogether.

## Cost Impact

- **Ledger entry accesses & I/O bytes:** Unnecessary storage writes consume ledger entry write slots, write I/O bytes, and incur rent and state expansion fees without providing any functional benefit.
