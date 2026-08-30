---
description: Storage read of a key that is never written anywhere in the crate — flag reads that always miss and waste metered storage access
sidebar_position: 5
---

# `storage_read_never_written`

| Default Severity | Category     |
| ---------------- | ------------ |
| `warn`           | StorageOperations |

## What it does

Accumulates every statically-known storage **read** key and every statically-known storage **write** key across the whole crate, then reports (at the end of the crate) each read whose key is written nowhere in that crate. A key read at runtime that is never written returns `None` on every invocation, so the contract pays for a full metered storage access that can never succeed.

## Why is this bad

A read that always misses still costs a full metered storage access on every invocation, forever. Three real causes sit behind it:

1. **Paying for a read that always misses** — the entry is simply never written.
2. **Depending on deleted or changed state** — a contract reading state that another contract used to write but no longer does.
3. **A key-name typo** — a misspelled key silently splits one logical entry into two, so the contract behaves as though the value was never set on a path the author believes is impossible. This last case is close to undiagnosable at runtime.

## Heuristic caveat (read this before acting)

This lint is **heuristic**, and its message says so. It works within a single crate and cannot see:

- **Cross-contract state sharing** — a factory/registry/token pattern where contract *A* reads a key that contract *B* (a different deployment) writes. This is the single most common false positive and is often completely correct by design.
- **Dynamically constructed keys** — when a key is built from a parameter or computed value, its value is unknown statically, so the read does not fire and does not suppress findings about unrelated static keys.

Because of this, the lint defaults to `warn`, not `deny`. Treat a hit as a prompt to confirm the key is initialised where expected, not as proof of a bug.

## Example

**Bad** — reading a key that is never written anywhere in the crate:

```rust
// ❌ Triggers: storage_read_never_written
fn balance(env: Env) -> Option<i32> {
    env.storage().persistent().get(&"total_supply") // never written in this crate
}
```

**Good** — the key is written somewhere in the crate (possibly a different function or module):

```rust
// ✅ No warning: the key is written elsewhere in the crate
fn init(env: Env) {
    env.storage().persistent().set(&"total_supply", &1_000);
}

fn balance(env: Env) -> Option<i32> {
    env.storage().persistent().get(&"total_supply")
}
```

**Acceptable false positive** — the key is written by another contract:

```rust
// ⚠️ Heuristic warning; suppress with #[allow(storage_read_never_written)]
// if `admin` is set by a separate deployment this contract reads from.
fn admin(env: Env) -> Option<Address> {
    env.storage().persistent().get(&"admin")
}
```

## Fix

Confirm the key is initialised where expected. If the write lives in another contract (cross-contract state sharing) or the key is constructed dynamically, suppress at the read site or crate level with `#[allow(storage_read_never_written)]`.
