# `address_equality_via_to_string`

**Default Severity:** `warn`

**Target Resource:** Memory — avoid allocating strings merely to compare address values.

## What it does

Flags `==` and `!=` comparisons where both operands are `to_string()` calls on `soroban_sdk::Address`.

## Why is this bad?

Converting each address to a string allocates two host strings and compares their bytes. `Address` already implements equality as a direct host comparison, so the conversion and allocations are unnecessary.

## Example

```rust
if a.to_string() == b.to_string() {
    // ...
}
```

Compare the handles directly instead:

```rust
if a == b {
    // ...
}
```

The same rewrite preserves `!=` comparisons. This lint only reports when both sides are address conversions; direct `Address` comparisons, `String` comparisons, and one-sided conversions are not reported.

Use `#[allow(address_equality_via_to_string)]` only when string-form comparison is intentional.
