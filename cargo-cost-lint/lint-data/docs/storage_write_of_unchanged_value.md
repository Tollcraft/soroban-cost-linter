---
description: Storage write of unchanged value — writing a read value back to storage without modification
---

# `storage_write_of_unchanged_value`

| Default Severity | Category |
| ---------------- | -------- |
| `warn`           | Storage  |

## What it does

Flags `set` calls on a storage accessor (`instance`, `persistent`, `temporary`) that unconditionally write back a value previously read from the same storage key without modifying it.

## Why is this bad

Reading a storage key, leaving the value unchanged, and writing it back re-encodes and rewrites the ledger entry, paying a metered storage write fee and entry rent for an identical value. A comparison guard (`if old_val != new_val`) or avoiding redundant writes avoids this cost.

## Example

**Bad** — writing back an unchanged value to storage:

```rust
// ❌ Triggers: storage_write_of_unchanged_value
fn update_balance(env: Env, key: &Symbol) {
    let balance: u64 = env.storage().instance().get(key).unwrap();
    // ... logic that does not modify balance ...
    env.storage().instance().set(key, &balance);
}
```

**Good** — only write when the value has changed:

```rust
// ✅ Fixed: guard the write with an inequality check or update the value
fn update_balance(env: Env, key: &Symbol, new_balance: u64) {
    let balance: u64 = env.storage().instance().get(key).unwrap();
    if balance != new_balance {
        env.storage().instance().set(key, &new_balance);
    }
}
```

## Fix

Add an inequality check `if old_val != new_val` before writing, or omit the write call if the value is not modified.
