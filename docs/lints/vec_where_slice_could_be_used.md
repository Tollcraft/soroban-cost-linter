# `vec_where_slice_could_be_used`

**Default Severity:** `warn`

**Target Resource:** [Memory — host-side allocation overhead](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects function parameters of type `soroban_sdk::Vec<T>` passed by value (`Vec<T>`, not `&Vec<T>` or `&mut Vec<T>`) where the function body never mutates the `Vec`. In these cases a native Rust slice (`&[T]`) would suffice.

## Why is this bad?

`soroban_sdk::Vec` is a host-side container. Creating one allocates memory on the host, and every read operation (`get`, `len`, iteration) crosses the Wasm/host boundary and consumes metered CPU budget. Passing a `Vec` by value also transfers ownership to the function, which means the caller must have a fully materialized host-side container even when it only needs to read a few elements.

Native Rust slices (`&[T]`) live entirely in Wasm memory. Reading from them costs nothing in terms of Soroban resource metering, and they can be backed by any contiguous memory — arrays, native `Vec`, or borrowed sub-slices — without requiring a host-side allocation.

## Example

```rust
use soroban_sdk::Vec;

// ❌ Bad: The function only reads from the Vec but takes it by value,
//          forcing the caller to create a host-side container.
fn bad(items: Vec) -> i32 {
    items.get(0)
}
```

```rust
// ✅ Good: Use a native Rust slice for read-only access.
//          The caller can pass any slice-backed data without host allocation.
fn good(items: &[i32]) -> i32 {
    items[0]
}
```

## When not to use a slice

If your function mutates the `Vec` — for example, calling `push_back`, `insert`, or passing it to another function that takes ownership — then a slice is not appropriate and the lint will not fire.

```rust
// ✅ Good: The Vec is mutated, so by-value ownership is necessary.
fn good_mutates(mut items: Vec) {
    items.push_back(42);
    // ...
}
```

```rust
// ✅ Good: The function passes the Vec elsewhere by value.
fn good_consumes(items: Vec, other: &mut SomeStruct) {
    other.store(items);
}
```

## Suggested Fix

- Change the parameter type from `soroban_sdk::Vec<T>` to a native Rust reference such as `&[T]` or `&Vec<T>`.
- Update callers to pass native Rust collections or slices instead of creating a host-side `Vec`.
- If the caller currently creates a `soroban_sdk::Vec` solely to pass it to this function, remove that host-side allocation entirely and use native Rust data structures.
- **Note:** `soroban_sdk::Vec` does not implement `Deref<Target=[T]>`, so you cannot directly pass an SDK `Vec` where a `&[T]` is expected. The caller must extract the elements into a native container first, or the function should be redesigned to work with native Rust types for internal operations.

## What is not reported

- Parameters of type `&Vec<T>` or `&mut Vec<T>` — they already express borrowed intent.
- Parameters that are mutated anywhere in the function body (including `push_back`, `insert`, or passed to other functions that may mutate).
- Parameters whose type does not resolve to `soroban_sdk::Vec`.
