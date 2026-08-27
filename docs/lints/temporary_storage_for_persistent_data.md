# `temporary_storage_for_persistent_data`

**Default Severity:** `warn`

**Target Resource:** [Entry Lifecycle — temporary storage durability](../cost_rationale.md)

## What it does

Flags code that writes a key to `env.storage().temporary()` and then, later in
the same function body, reads that same key back with `.unwrap()` or
`.expect()` — an *unchecked* read that assumes the value still exists.

Temporary storage is cheap, but it is not durable. A temporary entry is
**permanently deleted** once its Time-to-Live (TTL) expires — it is *not*
archived, so it can never be restored. Any contract that treats a temporary
value as though it will always be there is relying on data that can simply
disappear.

## Why is this bad?

{% hint style="danger" %}
Per the Soroban SDK and Stellar protocol documentation, temporary storage is
the cheapest storage option and is intended only for data that is relevant for
a short, well-defined period of time, or that **can be arbitrarily recreated**:

> Whenever a `TemporaryEntry` expires, the entry is permanently deleted and
> cannot be recovered... This storage type is best for entries that are only
> relevant for short periods of time or for entries that can be arbitrarily
> recreated.

When a temporary entry expires, it is removed from the ledger forever. A
`.unwrap()` or `.expect()` on a `get` of that key assumes the value is still
present; if the entry has expired, the unchecked read **panics**, wasting every
metered host call already performed and aborting the invocation. If the same
key is instead treated as guaranteed-to-persist for a balance or other
non-recreatable value, the data loss is permanent and unrecoverable.

Data that must survive across ledger closes belongs in `persistent()` or
`instance()` storage, whose entries are *archived* (not deleted) on expiry and
can be restored — so they behave "as if" they were stored forever.
{% endhint %}

## Example

```rust
use soroban_sdk::Env;

// ❌ Bad: writes to temporary storage, then assumes the entry still exists.
fn bad(env: Env, key: i32, balance: i128) -> i128 {
    env.storage().temporary().set(&key, &balance);
    env.storage().temporary().get::<_, i128>(&key).unwrap()
}
```

```rust
use soroban_sdk::Env;

// ✅ Good: absence is handled; a missing/expired entry has a safe default.
fn good(env: Env, key: i32, balance: i128) -> i128 {
    env.storage().temporary().set(&key, &balance);
    env.storage().temporary().get::<_, i128>(&key).unwrap_or(balance)
}

// ✅ Also good: match handles the None case explicitly.
fn good_match(env: Env, key: i32, balance: i128) -> i128 {
    match env.storage().temporary().get::<_, i128>(&key) {
        Some(v) => v,
        None => balance,
    }
}
```

## Suggested Fix

{% hint style="success" %}
Handle the `None` case the `get` returns — with `match`, `unwrap_or`,
`unwrap_or_else`, or an explicit `has()`-guarded read — so that an expired
entry is caught instead of panicking. Use temporary storage only for data that
is genuinely re-creatable; if the data must survive across ledger closes, write
it to `persistent()` or `instance()` storage instead.
{% endhint %}

## What is not reported

- Reads that handle absence: `unwrap_or`, `unwrap_or_else`, an explicit
  `match` on the returned `Option`, or a read guarded by a `has()` check that
  does not `unwrap`/`expect`.
- **Cache-like usage:** temporary storage intentionally used as a cache where
  a missing entry simply triggers recomputation/refetch (see
  [Handling False Positives](../false_positives.md)).
- Writes/reads on `persistent()` or `instance()` storage — those storage types
  are out of scope for this lint.
- Reads of a key that was never written to temporary storage in the same
  function.
