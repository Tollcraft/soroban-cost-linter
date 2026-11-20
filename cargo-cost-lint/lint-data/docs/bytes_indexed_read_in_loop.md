# `bytes_indexed_read_in_loop`

**Default Severity:** `warn`

**Target Resource:** Compute — repeated host reads and bounds checks.

## What it does

Flags calls to `get` on `soroban_sdk::Bytes` inside `for`, `while`, or `loop` bodies.

## Why is this bad?

Reading each byte with `Bytes::get` performs a metered host read and bounds check for every index. A per-byte traversal therefore crosses the host boundary once per element rather than copying the payload once.

## Example

```rust
for i in 0..bytes.len() {
    process(bytes.get(i).unwrap());
}
```

Copy the data once and index a native buffer, or iterate the payload without issuing one host read per byte. The lint is specific to Soroban `Bytes`; `Vec::get`, `Map::get`, reads outside loops, and calls under `#[allow(bytes_indexed_read_in_loop)]` are not reported.
