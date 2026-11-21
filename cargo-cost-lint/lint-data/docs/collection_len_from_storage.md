# `collection_len_from_storage`

**Default Severity:** `warn`

**Target Resource:** [Storage — ledger entry reads and host decoding](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags a storage `get` whose result is immediately unwrapped with `unwrap()` or
`expect()` and then used only by `len()` or `is_empty()`, when the stored value
is a Soroban `Vec`, `Map` or `Bytes`.

## Why is this bad?

{% hint style="danger" %}
A stored collection is decoded in full by `get`, including every element, even when the only question is whether the collection has any elements. That pays a metered ledger read plus the host-side decode cost for data that is immediately discarded, and the same cost is paid on every call that asks the question. `has` on the key, or a maintained counter, answers the same question without loading the collection.
{% endhint %}

## Example

❌ Bad:

```rust
let items: Vec<u32> = env.storage().instance().get(&key).unwrap();
if items.len() > 0 {
    // ...
}
```

✅ Good:

```rust
if env.storage().instance().has(&key) {
    // ...
}
```

Or keep a fixed-width counter next to the collection and read that instead.

## Suggested Fix

{% hint style="success" %}
Use `has(&key)` when the question is only "is there anything stored here?", or maintain a stored counter when the exact length is needed repeatedly. Keep the full `get` for call sites that actually consume the collection's elements.
{% endhint %}

## What is not reported

- A `get` whose value is bound to a local and used later; only the immediate
  `get` → `unwrap`/`expect` → `len`/`is_empty` parent chain is recognized.
- Uses of the value other than `len`/`is_empty`.
- Scalar results and collection types the lint cannot identify as a Soroban
  container.
- `Option` values that did not come from a Soroban storage accessor.

## Known limitations

The check is purely structural. A wrapper that unwraps the option for you, or
a helper that returns the length, is not followed across function boundaries.
