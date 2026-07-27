---
description: Map insert inside a loop — flag Map::insert calls inside loops that drive up storage read/write costs unnecessarily
sidebar_position: 6
---

# `map_insert_in_loop`

| Default Severity | Category     |
| ---------------- | ------------ |
| `warn`           | StorageOperations |

## What It Catches

Calling `Map::insert()` inside a loop incurs storage read/write costs on every iteration. If the same map is being modified repeatedly, the costs accumulate quickly.

## Example

**Bad** — inserting into a Map on every loop iteration:

```rust
// ❌ Triggers: map_insert_in_loop
fn populate(env: Env) {
    let mut map = Map;
    for i in 0..10 {
        map.insert(&i, &1); // Should Warn
    }
}
```

**Good** — collect items in memory first, then insert once:

```rust
// ✅ Fixed: accumulate in Vec, insert once
fn populate(env: Env) {
    let mut entries = Vec::new();
    for i in 0..10 {
        entries.push((i, 1u32));
    }
    let mut map = Map;
    for (k, v) in entries {
        map.insert(&k, &v);
    }
}
```

## Fix

Accumulate entries in a `Vec` or similar in-memory structure during the loop, then perform a single Map construction or batch insert after the loop.