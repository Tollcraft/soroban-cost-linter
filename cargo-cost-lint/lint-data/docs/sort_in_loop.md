# `sort_in_loop`

**Default Severity:** `warn`

**Target Resource:** Compute — repeated sorting work on a host collection.

## What it does

Flags `sort()` and `sort_by()` calls on `soroban_sdk::Vec` inside `for`, `while`, or `loop` bodies.

## Why is this bad?

Sorting a host collection performs metered work across the VM boundary. Repeating the sort on each loop iteration can multiply that work even when the collection contents do not change.

## Example

```rust
for item in items {
    values.sort();
    process(item, &values);
}
```

When the collection contents are loop-invariant, sort once before the loop:

```rust
values.sort();
for item in items {
    process(item, &values);
}
```

Sorting outside loops and sorting native Rust slices are not reported. If sorting is required after each iteration because the contents change, suppress the intentional call with `#[allow(sort_in_loop)]`.
