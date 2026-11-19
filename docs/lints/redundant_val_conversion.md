# `redundant_val_conversion`

**Default Severity:** `warn`

**Target Resource:** [CPU — metered host calls across the native-Rust/Val boundary](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects conversions that cross the Soroban native-Rust/`Val` boundary without
producing anything new:

1. **Same-type conversion** — a value converted into the type it already is
   (e.g. a `u32` `.into_val()`'d back to a `u32`).
2. **Round trip** — a value converted into `Val` and immediately converted back
   to its original type within the same expression chain (e.g.
   `u32::try_from_val(env, &num.into_val(env))`).

## Why is this bad?

{% hint style="danger" %}
Crossing the `Val` boundary is a *metered* operation: each `into_val` /
`try_from_val` call is a host call that the network charges for. A conversion
that lands on the same type — or that goes `T -> Val -> T` in one shot — pays
for two boundary crossings to hand back the value it started with. These add up
quietly when values are passed between helper functions, which is exactly the
structurally-expensive, input-independent pattern this linter exists to catch.
{% endhint %}

## Cost impact

Each redundant hop is a metered host call (and, for `try_from_val`, the
`Result` unwrap path) that contributes CPU budget with zero behavioural
benefit. The cost is independent of input, so it is pure, repeatable waste on
every invocation — see the [Cost Rationale — Metered
Resources](../cost_rationale.md#1-cpu-instructions) for the cost types involved.

## Example

```rust
// ❌ Bad: converting a u32 into a u32
let same: u32 = num.into_val(&env);

// ❌ Bad: round trip through Val
let same: u32 = u32::try_from_val(&env, &num.into_val(&env)).unwrap();
```

## Known False Positives (Not Flagged)

The lint compares the *concrete source and target types* through `LateContext`,
so it deliberately stays silent in the following situations:

1. **Generic contexts** — a conversion like `t.into_val(env)` inside a generic
   helper `fn f<T: IntoVal<Env, T>>(t: T) -> T` is *not* flagged. The source and
   target line up only because the type parameter happens to be equal; that is
   not a real round trip and reporting it would be a false positive.
2. **Inference-variable types** — when either side is still an unresolved
   inference variable (a conversion that merely pins an inference variable),
   the lint defers rather than guessing.

```rust
// ✅ Not flagged: generic helper, the equality is incidental
fn pass_through<T: IntoVal<Env, T>>(t: T, env: &Env) -> T {
    t.into_val(env)
}
```

## Suggested Fix

{% hint style="success" %}
Remove the redundant conversion and use the original value directly, or convert
only once to the type you actually need to send across the boundary.
{% endhint %}
