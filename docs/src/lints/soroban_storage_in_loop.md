# `soroban_storage_in_loop`

**Default Severity:** Warn

## What it does
Detects storage operations (reads or writes) that are executed inside loop bodies (`for`, `while`, or `loop`).

## Why is this bad?
Storage operations in Soroban are the most expensive resource. Placing them inside a loop wastes CPU instructions and network read/write throughput, drastically increasing your transaction fees.

## Example
```rust
for item in items {
    env.storage().instance().set(&item, &1); // Bad
}
```

## Suggested Fix
Accumulate mutations in memory (using a `Vec` or `Map`) during the loop execution, and perform a single storage read/write operation outside of the loop.
