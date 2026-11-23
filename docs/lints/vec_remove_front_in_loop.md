# `vec_remove_front_in_loop`

**Default Severity:** `warn`

**Target Resource:** [Compute — repeated element copies](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags `Vec::remove(0)` on a `soroban_sdk::Vec` inside a syntactic `for`, `while`
or `loop` body.

## Why is this bad?

{% hint style="danger" %}
Removing index `0` shifts every remaining element one slot to the left. Used as
a drain loop, that turns an O(n) operation into O(n^2) host-side copies and
repeated metered collection writes.
{% endhint %}

## Example

❌ Bad:

```rust
while !values.is_empty() {
    let first = values.remove(0);
    // ...
}
```

✅ Good:

```rust
let native: std::vec::Vec<_> = values.iter().collect();
let first = native.first();
let rebuilt = Vec::from_array(&env, native.try_into().unwrap());
```

## Suggested Fix

{% hint style="success" %}
Accumulate the drained values in a native `Vec` and build a single Soroban `Vec`
afterwards. If order does not matter, `swap_remove` removes the last element in
constant time instead.
{% endhint %}

## What is not reported

- `remove` with a symbolic or non-zero index.
- A front removal outside a loop.
- The same call on a native `std::vec::Vec`.
- Calls suppressed with `#[allow(vec_remove_front_in_loop)]`.
