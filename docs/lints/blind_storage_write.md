---
description: Blind storage write — overwrite of a previously written key without reading it back
sidebar_position: 5
---

# `blind_storage_write`

| Default Severity | Category     |
| ---------------- | ------------ |
| `warn`           | StorageOperations |

## What it does

Flags `set` calls on a storage accessor (`instance`, `persistent`, `temporary`) that overwrite a key which was **already written earlier in the same function**, when the code in between never read that key's value back. The overwrite silently discards the previous store.

This lint is distinct from [`storage_write_without_read`](storage_write_without_read.md): that lint fires when a key is **never** read anywhere in the function (the written value is unused), whereas `blind_storage_write` fires only when the key **is** read somewhere in the function — so the write is plausibly meaningful — but this specific overwrite happens without consulting the prior value.

## Why is this bad

Each `set` pays a Soroban storage write fee and consumes CPU/memory budget. When you write a key, then write it again without having read the first value, the first write was wasted: its result is discarded before anything could observe it. This is a classic "blind overwrite" that usually indicates either dead work or a missing read (e.g. you meant to update a derived value based on the existing one).

## Example

**Bad** — writing the same key twice in one function without reading it back between writes:

```rust
// ❌ Triggers: blind_storage_write
fn refresh(env: Env, key: &str) {
    let previous: Option<i32> = env.storage().instance().get(key); // read happens elsewhere
    env.storage().instance().set(key, &1);
    // ... later, blind overwrite: the earlier `set` is discarded ...
    env.storage().instance().set(key, &2);
}
```

**Good** — read the value back immediately before overwriting so the write is informed:

```rust
// ✅ Fixed: the second write consults the current value
fn refresh(env: Env, key: &str) {
    let previous: Option<i32> = env.storage().instance().get(key);
    env.storage().instance().set(key, &1);
    let current: Option<i32> = env.storage().instance().get(key); // informed overwrite
    env.storage().instance().set(key, &compute_next(current));
}
```

**Also fine** — initialising a brand-new key with a single `set` is never flagged (there is no prior write to discard):

```rust
// ✅ No warning: first/only write to a new key
fn init(env: Env, key: &str) {
    env.storage().instance().set(key, &1);
}
```

## Fix

Before overwriting a key that you have already written in the same function, read its current value (via `get`/`try_get`/`has`) so the new value derives from it. If the prior write is genuinely dead, remove it instead of overwriting.
