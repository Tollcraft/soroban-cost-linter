---
description: Map insert in loop — avoid inserting elements into maps inside loop bodies
sidebar_position: 10
---

# `map_insert_in_loop`

| Default Severity | Category     |
| ---------------- | ------------ |
| `warn`           | Compute      |

## What it does

Flags `Map::insert` calls executed inside loop bodies.

## Why is this bad

Inserting elements into a Soroban `Map` inside a loop repeatedly invokes host object mutation and re-allocations per iteration, driving up CPU costs.

## Example

**Bad** — inserting into a map inside a loop:

```rust
// ❌ Triggers: map_insert_in_loop
let mut map = Map::new(&env);
for (k, v) in &items {
    map.set(*k, *v);
}
```

**Good** — populate or construct the map outside or via batch initialization:

```rust
// ✅ Fixed: construct map outside loop or batch insertions
let mut map = Map::new(&env);
// Populate items...
```

## Suggested Fix

Avoid calling `.set()` or `.insert()` on host maps inside loop iterations where possible. Pre-populate or aggregate outside the loop.

## Cost Impact

- **CPU instructions:** Repeated host map insertions incur continuous host function dispatch overhead and entry reallocation costs, significantly inflating CPU instruction consumption.
