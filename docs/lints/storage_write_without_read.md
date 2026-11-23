---
description: Storage write without corresponding read — flag unnecessary writes that waste Soroban storage fees
sidebar_position: 20
---

# `storage_write_without_read`

| Default Severity | Category     |
| ---------------- | ------------ |
| `warn`           | StorageOperations |

## What it does

Flags storage writes (`.set()`) on a storage object when the same key was not observed first on the same storage object. Any of `.get()`, `.try_get()`, `.has()`, `.remove()` or `.update()` earlier in the function body counts as observing the key, so a `.set()` after one of them is not reported.

## Initialiser exemption

Functions whose name marks them as initialisers are skipped entirely, because writing before any read is expected there. The rule is a plain substring test on the function name:

```rust
fn_name.contains("init") || fn_name.contains("set_admin")
```

So `initialize`, `init_owner`, `set_admin` and `set_admin_key` are all exempt by one rule.

> **Warning:** because the match is a substring test, it also exempts unrelated functions whose names happen to contain `init` or `set_admin`. For example, `reinitialise_cache` contains `init` and is never reported, even when it writes a key it never read. This false negative is pinned by the `reinitialise_cache` case in the UI fixture.

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
