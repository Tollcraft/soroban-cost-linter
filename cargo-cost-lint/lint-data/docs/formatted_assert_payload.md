# `formatted_assert_payload`

**Default Severity:** `warn`

**Target Resource:** [Compute — formatting instructions](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags `assert!`, `debug_assert!`, `assert_eq!` and `assert_ne!` calls whose
failure message contains formatting arguments.

## Why is this bad?

{% hint style="danger" %}
A formatted assertion expands to `core::fmt` machinery that is linked into the
contract and runs on every check, even though the message is only needed when
the assertion fails. On a failing path a formatted message is also strictly
more expensive than a bare contract error code.
{% endhint %}

## Example

❌ Bad:

```rust
assert!(balance >= minimum, "balance {} below {}", balance, minimum);
assert_eq!(left, right, "mismatch for key {}", key);
```

✅ Good:

```rust
assert!(balance >= minimum);
assert_eq!(left, right);
```

Or, for an error the caller needs to distinguish:

```rust
if balance < minimum {
    panic_with_error!(env, Error::InsufficientBalance);
}
```

## Suggested Fix

{% hint style="success" %}
Use a plain assertion when the message carries no information beyond the
failed condition. For contract errors, use `panic_with_error!` with a
`#[contracterror]` enum: it compiles to a plain integer error code with no
formatting machinery.
{% endhint %}

## What is not reported

- `assert!(condition)` with no custom message.
- An assertion whose message is a plain string literal with no arguments.
- Code under `#[cfg(test)]`.
- Calls suppressed with `#[allow(formatted_assert_payload)]`.
