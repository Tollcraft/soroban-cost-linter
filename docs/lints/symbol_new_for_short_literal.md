# `symbol_new_for_short_literal`

**Default Severity:** `warn`

## What it does

Detects calls to `Symbol::new(&env, "literal")` where the string literal is short enough (≤ 9 characters) and contains only valid short-symbol characters (`a-zA-Z0-9_`). Such symbols can be created at compile time using the `symbol_short!` macro instead.

## Why is this bad?

{% hint style="danger" %}
`Symbol::new` creates symbols at runtime, which incurs CPU overhead on every call. Short symbols (≤ 9 chars, alphanumeric + underscore) can be created at **compile time** using `symbol_short!`, producing a `const Symbol` with zero runtime cost.
{% endhint %}

## Example

```rust
// ❌ Bad: runtime symbol creation for a short literal
let sym = Symbol::new(&env, "hello");
```

## Suggested Fix

{% hint style="success" %}
Use `symbol_short!` macro for compile-time symbol creation:
{% endhint %}

```rust
// ✅ Good: compile-time symbol creation
let sym = symbol_short!("hello");
```

## Cost impact

Every `Symbol::new(&env, "literal")` call crosses the Wasm–host boundary to allocate and register the symbol at runtime — even for short, compile-time-knowable literals. `symbol_short!` produces a `const Symbol` with **zero runtime cost**.

Measured with `Env::default()` in the [`cost_benchmarks`](https://github.com/Tollcraft/soroban-cost-linter/tree/main/cost_benchmarks) crate (`cargo test -- --nocapture`):

| Pattern | Iterations | CPU instructions (delta) | Memory bytes (delta) |
| --- | --- | --- | --- |
| `Symbol::new(&env, "hello")` (bad) | 100 | *run `cargo test -- --nocapture` in `cost_benchmarks/`* | *run `cargo test -- --nocapture` in `cost_benchmarks/`* |
| `symbol_short!("hello")` (good) | 100 | ≈ 0 (compile-time constant) | ≈ 0 (compile-time constant) |

{% hint style="info" %}
The saving per call is small in absolute terms, but many contracts create dozens of symbols at init. Using `symbol_short!` where possible eliminates every one of those host crossings.
{% endhint %}

### How to reproduce

```bash
cd cost_benchmarks
cargo test bench_symbol_new_vs_short -- --nocapture
```

The test calls each pattern 100 times and prints the budget delta.

## Valid Characters and Length

- Maximum length: **9 characters**
- Valid characters: `a-z`, `A-Z`, `0-9`, `_` (underscore)

## Non-Flagged Cases

The lint does **not** trigger for:
- String literals longer than 9 characters
- String literals containing invalid characters (e.g., `-`, `.`, spaces)
- Non-literal string arguments (variables, expressions)
- Empty strings