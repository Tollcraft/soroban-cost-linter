---
description: Inefficient bytes concatenation — flag repeated Bytes concatenation in loops that creates unnecessary allocations
sidebar_position: 5
---

# `inefficient_bytes_concat`

| Default Severity | Category   |
| ---------------- | ---------- |
| `warn`           | Memory     |

## What it does

Flags inefficient Bytes concatenation inside a loop using the `+` operator, which creates a new allocation on every iteration.

## Why is this bad

Using the `+` operator to concatenate `Bytes` values inside a loop creates a new allocation on every iteration, which is both CPU-expensive and wasteful of Soroban memory charges.

## Example

**Bad** — re-allocating a new `Bytes` value on every loop iteration:

```rust
// ❌ Triggers: inefficient_bytes_concat
fn build_message(env: Env) {
    let mut result = Bytes::from("");
    for _ in 0..10 {
        result = result + Bytes::from("x");
    }
}
```

**Good** — accumulate in a `Vec<u8>` buffer and convert once:

```rust
// ✅ Fixed: use Vec<u8> buffer, then convert
fn build_message(env: Env) {
    let mut buf = Vec::new();
    for _ in 0..10 {
        buf.extend_from_slice(b"x");
    }
    let _result = Bytes::from(&buf[..]);
}
```

## Fix

Replace `+` concatenation of `Bytes` values with a `Vec<u8>` buffer that accumulates bytes, then convert to `Bytes` once after the loop using `Bytes::from`.
