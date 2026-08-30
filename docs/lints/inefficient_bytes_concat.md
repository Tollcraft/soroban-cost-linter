---
description: Inefficient bytes concatenation — avoid repeated per-iteration concatenations that waste CPU and memory
sidebar_position: 6
---

# `inefficient_bytes_concat`

| Default Severity | Category     |
| ---------------- | ------------ |
| `warn`           | Compute      |

## What it does

Flags repeated concatenation of `Bytes` or `BytesN` objects inside loop bodies using operator-based concatenation.

## Why is this bad

Concatenating bytes repeatedly inside a loop allocates new host objects and copies backing buffers on every iteration, causing quadratic performance degradation and excessive CPU consumption.

## Example

**Bad** — concatenating bytes inside a loop:

```rust
// ❌ Triggers: inefficient_bytes_concat
let mut result = Bytes::new(&env);
for item in &items {
    result = result + item;
}
```

**Good** — preallocate or accumulate using a vector or buffer before converting to host bytes:

```rust
// ✅ Fixed: collect/accumulate outside the loop
let mut buffer = Vec::new();
for item in &items {
    buffer.extend_from_slice(item.to_alloc_vec());
}
let result = Bytes::from_slice(&env, &buffer);
```

## Suggested Fix

Replace `+` concatenation of `Bytes` values with a `Vec<u8>` buffer that accumulates bytes, then convert to `Bytes` once after the loop using `Bytes::from`.

## Relationship to other lints

Note that this lint (`inefficient_bytes_concat`) specifically flags the binary `+` operator (`b1 + b2`). A related lint, `soroban_inefficient_bytes_concat`, flags the `.push_back()` and `.append()` method calls on `Bytes`. They detect genuinely different code shapes, but both enforce the same best practice: accumulate bytes in a `Vec<u8>` instead of using host objects in a loop.
