# `string_from_str_in_loop`

**Default Severity:** `warn`

**Target Resource:** [Host — repeated host allocations](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags `soroban_sdk::String::from_str(&env, value)` inside a syntactic `for`,
`while` or `loop` body when `value` is a literal or a `const`.

## Why is this bad?

{% hint style="danger" %}
`String::from_str` encodes and validates its input through the host and returns
a fresh host `String`. When the input is fixed at compile time, every iteration
repeats that metered work to produce the same value.
{% endhint %}

## Example

❌ Bad:

```rust
for item in items {
    let label = String::from_str(&env, "label");
    // ...
}
```

✅ Good:

```rust
let label = String::from_str(&env, "label");
for item in items {
    // ...reuse label
}
```

## Suggested Fix

{% hint style="success" %}
Build the `String` once before the loop and reuse it. For short fixed keys, a
`Symbol` or fixed-width integer avoids the allocation entirely.
{% endhint %}

## What is not reported

- A `from_str` call whose value is computed at runtime.
- The same call outside a loop.
- A same-named associated function on a local type.
- Calls suppressed with `#[allow(string_from_str_in_loop)]`.
