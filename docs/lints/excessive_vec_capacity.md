# `excessive_vec_capacity`

**Default Severity:** `warn`

**Target Resource:** [Memory](../cost_rationale.md#2-memory-ram)

## What it does

Detects calls to `soroban_sdk::vec::Vec::with_capacity(n)` and
`.reserve(n)` where the capacity argument is a hard-coded literal that
exceeds a defensible threshold (currently **4 096 elements**).

## Why is this bad?

Soroban contracts execute inside a WASM linear-memory sandbox with a
hard per-transaction memory cap. Each element in a Soroban `Vec` is
metered individually — pre-allocating far more capacity than the
collection will actually hold wastes host memory and inflates the
metered cost of the allocation without providing a meaningful
performance benefit for typical Soroban workloads.

A capacity of 4 096 elements (roughly 16 KB for4-byte values) is
generous for known-bound workloads while remaining well within the
per-transaction memory cap. Values above this threshold are almost
certainly over-estimated and should be revisited.

## Threshold rationale

The threshold of **4 096 elements** is based on Soroban's metered
memory model:

1. **Memory is metered per-element.** Each element consumes
   individually-metered host memory, so unnecessary pre-allocation
   is not free.

2. **Hard memory cap.** Contracts run in a WASM sandbox with a
   per-transaction memory limit. Excessive pre-allocation brings
   the contract closer to this limit for no benefit.

3. **Conservative default.** 4 096 is large enough to cover
   legitimate use cases (fixed-size buffers, small batch processing)
   while catching wasteful patterns (pre-allocating 100K+ elements
   for small datasets).

4. **No performance benefit.** Unlike `std::vec::Vec`, Soroban's
   Host-backed Vec does not benefit from Rust-side pre-allocation
   the same way — elements are metered on the host side regardless.

## Example

```rust
// Bad: wasteful pre-allocation
let _v = Vec::with_capacity(1_000_000);

// Good: start empty and let growth happen naturally
let _v = Vec::new();

// Good: runtime-derived capacity is not flagged
let n = compute_needed_capacity();
let _v = Vec::with_capacity(n);
```

## Known limitations

- **Only detects hard-coded literals.** Runtime-derived capacities
  (variables, function calls, arithmetic expressions) are
  intentionally ignored because the lint cannot determine their
  value statically.
- **Only targets `soroban_sdk::vec::Vec`.** Ordinary host-side
  `std::vec::Vec` helper code is not flagged.
