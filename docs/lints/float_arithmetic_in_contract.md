# `float_arithmetic_in_contract`

**Default level:** `warn`

**Target Resource:** [CPU — Wasm instruction execution + software float emulation](../cost_rationale.md#per-lint-resource-summary)

## What is this?

This lint flags any arithmetic operation (`+`, `-`, `*`, `/`) performed on `f32` or `f64` values inside contract code.

## Why is this bad?

wasm32 has no hardware floating point available to Soroban contracts. Every `f32`/`f64` operation compiles to a **soft-float routine** — tens to hundreds of wasm instructions for what looks in source like a single multiply. A price calculation written with `f64` can cost orders of magnitude more CPU than the same calculation in fixed-point integers.

**Determinism is the stronger argument.** Floats are not deterministic across all rounding paths, and a contract whose result depends on float rounding is a contract whose consensus behaviour depends on it too. Flagging the pattern at compile time is considerably kinder than discovering it from a CPU budget failure on mainnet.

## Example

```rust
// BAD: floating-point arithmetic in contract code
fn calculate_price(env: &Env, rate: f64, quantity: f64) -> f64 {
    rate * quantity  // soft-float multiply: ~100 wasm instructions
}
```

```rust
// GOOD: fixed-point integer arithmetic
fn calculate_price(env: &Env, rate: i128, quantity: i128) -> i128 {
    rate * quantity  // native i128 multiply: a few wasm instructions
}
```

## Suggested Fix

Replace `f32`/`f64` types with fixed-point integer types (`i128`, `u128`, or a custom fixed-point wrapper). Use integer arithmetic with explicit scaling factors.

## Known False Positives

- **Non-contract helper code:** Functions outside of `#[contractimpl]` blocks (test utilities, build scripts, benchmarks) may legitimately use floats. The lint scope boundary is the `#[contractimpl]` attribute.
- **Explicit cast operations:** `f64::from_bits()` or `f64::to_bits()` for bit-level inspection do not perform arithmetic and are not flagged.

## Relationship to other lints

This lint is complementary to `u128_where_u64_suffices`, which catches expensive 128-bit integer arithmetic. Together they enforce the principle that contract arithmetic should use the cheapest integer type that fits the business logic.
