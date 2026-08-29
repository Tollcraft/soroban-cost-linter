# `val_conversion_chain`

**Default Severity:** `warn`

**Target Resource:** [CPU — host function dispatch and execution](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags a sequence of **three or more** `soroban_sdk` conversions that shuttle the
*same underlying local value* through `Val`, built up across `let` bindings (or
a tail expression) within a single block.

Soroban's native-Rust/`Val` boundary is crossed by the `IntoVal`, `TryIntoVal`,
`FromVal`, and `TryFromVal` trait methods (`into_val`, `try_into_val`,
`from_val`, `try_from_val`). Every one of those hops is a metered host call.
This lint follows a value through a chain such as:

```text
u32 ──into_val──▶ Val ──try_into_val──▶ Vec<u32> ──into_val──▶ Val ──try_into_val──▶ u32
```

and fires once the chain reaches the configured threshold.

The threshold is the documented named constant `VAL_CONVERSION_CHAIN_MIN_HOPS`
(declared in `soroban_cost_lints/src/lib.rs`), currently **3**. Two conversions
(`T -> Val -> U`) is the minimal round trip a single foreign API boundary
demands and is left to the sibling [`redundant_val_conversion`](redundant_val_conversion.md)
lint. Three or more means the value has been bounced through `Val` at least
twice, almost always because two helper functions each wanted a different shape
and the author bridged them at the call site — three metered host calls to move
one value where a direct `T -> U` conversion (or a single `into_val` to the
type the next helper actually needs) would have done it in one.

## Why is this bad?

{% hint style="danger" %}
Crossing the Wasm-guest/host boundary is one of the most expensive single
operations a contract performs: each `into_val` / `from_val` family call pays
`DispatchHostFunction` overhead plus the conversion work itself. A chain that
bounces through `Val` and back multiple times multiplies that cost for no
change in the *logical* value being carried — the same data merely changes
representation three or more times. See the [Cost Rationale — What
Dominates](../cost_rationale.md#what-dominates) for the relative cost hierarchy.
{% endhint %}

## Example

```rust
// ❌ Bad: four hops to round-trip one value through `Val`
fn bad_chain(env: &soroban_sdk::Env) {
    let base: u32 = 7;
    let v1 = base.into_val(env);                         // u32   -> Val
    let mid: soroban_sdk::Vec<u32> = v1.try_into_val(env).unwrap(); // Val -> Vec<u32>
    let v2 = mid.into_val(env);                          // Vec<u32> -> Val
    let _final: u32 = v2.try_into_val(env).unwrap();     // Val   -> u32
}
```

```rust
// ✅ Good: convert the value directly to the representation the next step needs
fn good_direct(env: &soroban_sdk::Env) {
    let base: u32 = 7;
    // Only the hops the API actually requires; no gratuitous round-trips.
    let mid: soroban_sdk::Vec<u32> = base.into_val(env);
    let _ = mid;
}
```

## Suggested Fix

{% hint style="success" %}
Convert the value **once**, directly into the type the consuming call expects,
instead of hopping `native -> Val -> other-native -> Val -> …`. If two helpers
demand different shapes, pick the intermediate representation that avoids
re-entering `Val`, or refactor one helper to accept the form you already have.
{% endhint %}

## Relationship to `redundant_val_conversion`

This lint is the **longer-sequence sibling** of `redundant_val_conversion`
(issue [#397](https://github.com/Tollcraft/soroban-cost-linter/issues/397)):

- `redundant_val_conversion` targets the **two-hop round trip inside a single
  expression** — e.g. `x.into_val(e).try_into_val(e)` that lands back where it
  started.
- `val_conversion_chain` targets the **three-or-more-hop chain that builds up
  across a statement or `let` sequence**, where no individual hop looks wrong but
  the sequence as a whole pays for every bounce.

The two are independent: `val_conversion_chain` is keyed on type information
through `LateContext` and fires whether or not `redundant_val_conversion` also
exists, and it does not assume the chain's start and end types are equal. A
single expression that triggers `redundant_val_conversion` is too short to
reach this lint's threshold, so the two never collide.

## What is not reported

- **Below the threshold.** One or two conversions of the same value are not
  long enough to fire (the minimal `T -> Val -> U` round trip is the
  territory of `redundant_val_conversion`).
- **Native-only conversions.** The blanket `impl IntoVal<E, U> for T where T:
  Into<U>` performs a cheap, host-free `Into` conversion. Because neither side
  is `Val`, those hops do not count toward the chain.
- **Different underlying values.** Each hop must consume the *same* local value
  produced by the previous hop. Two independent conversion streams that happen
  to sit in the same block do not form a chain.
- **Conversions required by a signature you do not control.** If a foreign
  function's signature forces a specific `Val` boundary crossing, that single
  hop is expected; only the *accumulation* of three or more on one value is
  flagged.
- **Genuine value changes.** A conversion that produces a semantically
  different value for the next step (not merely re-packaging the same data)
  should be reviewed on its own merits; this lint only points out needless
  multi-hop re-packaging.

## Known false positives

- Helper functions with fixed, non-negotiable `Val`/native signatures can make a
  long chain *look* gratuitous while each hop is individually required. Such
  cases should be silenced with `#[allow(val_conversion_chain)]` and, where
  possible, by adjusting the helper signatures.
