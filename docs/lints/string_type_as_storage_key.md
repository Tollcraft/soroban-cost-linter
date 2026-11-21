# `string_type_as_storage_key`

**Default Severity:** `warn`

**Target Resource:** [Storage — ledger entry keys and ledger I/O bytes](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags `get`, `has` and `set` on a Soroban storage accessor when the key is built
by `String::from_str(&env, "...")` from a short string literal that would fit a
`symbol_short!`.

## Why is this bad?

{% hint style="danger" %}
A `String` key carries its full byte length into every ledger entry. The key is encoded, hashed and stored on every access, so an unnecessarily wide key is paid for on every read and write for the life of the entry. A `Symbol` or a fixed-width integer encodes far less data for the same short, fixed set of names.
{% endhint %}

## Example

❌ Bad:

```rust
env.storage().instance().set(&String::from_str(&env, "count"), &1u32);
```

✅ Good:

```rust
env.storage().instance().set(&symbol_short!("count"), &1u32);
```

## Suggested Fix

{% hint style="success" %}
Replace short literal keys with `symbol_short!("name")` (up to nine ASCII alphanumeric/underscore characters) or with a fixed-width integer key. Keep `String` keys for names that are genuinely variable or too long for a symbol.
{% endhint %}

## When is this NOT flagged?

- Literals longer than nine characters or containing characters that are not
  valid in a short symbol, which the `is_valid_short_symbol` predicate rejects.
- `Symbol` and integer keys.
- A `String` passed as the stored *value* rather than as the key.
- Keys built at runtime from user input: without a literal there is no bounded
  suggestion, so the lint stays silent rather than guessing.
- Calls suppressed with `#[allow(string_type_as_storage_key)]`.

## Known limitations

The key expression must be a direct `String::from_str` call in the first
argument. A key computed in a local first, or built through a helper function,
is not followed.
