---
description: Persistent write without TTL extension — writing persistent storage without extending TTL
---

# `persistent_write_without_ttl_extension`

| Default Severity | Category |
| ---------------- | -------- |
| `warn`           | Storage  |

## What it does

Flags persistent storage writes (`env.storage().persistent().set(...)`) where `extend_ttl` is not invoked on the same storage key within the enclosing function.

## Why is this bad

Persistent ledger entries expire and become archived when their TTL runs out. Writing an entry without extending its TTL risks premature archival, which later forces an expensive metered restoration transaction.

## Example

**Bad** — writing persistent storage without extending its TTL:

```rust
// ❌ Triggers: persistent_write_without_ttl_extension
fn update_state(env: Env, key: &Symbol, value: &State) {
    env.storage().persistent().set(key, value);
}
```

**Good** — extending TTL after writing:

```rust
// ✅ Fixed: call extend_ttl with explicit threshold and extension length
fn update_state(env: Env, key: &Symbol, value: &State) {
    env.storage().persistent().set(key, value);
    env.storage().persistent().extend_ttl(key, 1000, 10000);
}
```

## Fix

Call `env.storage().persistent().extend_ttl(key, ...)` after writing to ensure the ledger entry remains active.
