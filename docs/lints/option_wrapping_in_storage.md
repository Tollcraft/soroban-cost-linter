# option_wrapping_in_storage

| Property | Value |
| --- | --- |
| Default severity | `warn` |
| Category | Storage Operations |

## What it does

Storing an `Option<T>` directly in Soroban storage when the storage key already models absence.

## Why is this bad?

Soroban storage already models absence: a missing key returns `None` from `get()`. Storing `Option<T>` creates a **three-state model**:

1. **Missing** — the key does not exist (returns `None`)
2. **`Some(T)`** — the key exists and holds a value
3. **`None`** — the key exists but holds `None`

State 2 and state 3 are semantically different but the contract likely does not intend this distinction. The extra `None` state wastes storage space and adds unnecessary serialization overhead.

## Triggering example

```rust
fn store_with_option(env: Env) {
    let val: Option<u32> = Some(42);
    env.storage().instance().set(&"key", &val); // warn: option_wrapping_in_storage
}
```

## Recommended rewrite

Store `T` directly and remove the key when you want to express absence:

```rust
fn store_without_option(env: Env) {
    let val: u32 = 42;
    env.storage().instance().set(&"key", &val); // clean — no Option wrapper
}
```

## When NOT to suppress

Do **not** use this lint as an excuse to remove `require_auth` or other security checks. This lint concerns storage data modeling, not authorization.

## When to suppress

If your contract intentionally needs a tri-state entry (missing, present-with-value, present-without-value), suppress the lint at the call site:

```rust
#[allow(option_wrapping_in_storage)]
env.storage().instance().set(&"key", &val);
```

See [false positives](../false_positives.md#option_wrapping_in_storage) for a documented tri-state use case.

## Implementation notes

This is a **type-aware** lint. It resolves the concrete type of the value argument to `.set()` and checks whether it is `Option<T>` at the top level. A struct containing an `Option` field (e.g. `MyStruct { value: Option<u32> }`) does **not** trigger — only a direct `Option<T>` stored value triggers.
