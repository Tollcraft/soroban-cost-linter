# `persistent_storage_for_ephemeral_data`

**Default Severity:** `warn`

**Target Resource:** [Entry Lifecycle — temporary storage durability](../cost_rationale.md)

## What it does

Flags code that writes a key to `env.storage().persistent()` and then removes
that same key on **every** path through the same function — so the entry can
never be observed after the invocation returns.

Temporary storage is the cheapest storage option and is intended for data that
is only relevant for a short, well-defined period, or that can be arbitrarily
recreated. A value that a function creates with `set` and tears down with
`remove` before returning is ephemeral by construction: no other invocation can
ever read it. Storing it in `persistent()` buys durability that is provably
never used.

## Why is this bad?

{% hint style="danger" %}
`Persistent` entries are the most expensive storage type in Soroban. They carry
rent payments to keep them alive and live under the archiving/restoration
semantics described in the
[storage-type durability table](../cost_rationale.md#entry-lifecycle--durability-temporary-vs-persistent-vs-instance).

When a value is written and then removed on every path through the same
function, the write can never outlive the call. Paying `persistent()`'s rent and
archival overhead for such a value is pure waste: the data is gone before the
ledger even closes, so the "durability" is never exercised.
{% endhint %}

## Example

```rust
use soroban_sdk::Env;

// ❌ Bad: the value is written to persistent() and then removed on every path,
// so it never survives the call — temporary() costs the same but avoids rent.
fn bad(env: Env, key: i32) {
    env.storage().persistent().set(&key, &42);
    env.storage().persistent().remove(&key);
}
```

```rust
use soroban_sdk::Env;

// ✅ Good: scratch data that is torn down before returning belongs in
// temporary() storage.
fn good(env: Env, key: i32) {
    env.storage().temporary().set(&key, &42);
    env.storage().temporary().remove(&key);
}

// ✅ Also good: data that must survive across invocations is not removed, so
// the lint (correctly) stays silent.
fn good_survives(env: Env, key: i32) -> i32 {
    env.storage().persistent().set(&key, &42);
    env.storage().persistent().get::<_, i32>(&key).unwrap_or(0)
}
```

## Suggested Fix

{% hint style="success" %}
Use `env.storage().temporary()` for any value that is removed on every path
through the function in which it is written. If the entry legitimately survives
across invocations, keep `persistent()` and ensure the removal is conditional —
a remove only on some paths (or of a different key) is never flagged.
{% endhint %}

## What is not reported

- Removals that are not provably on every path: an early `return` or `panic`
  that skips the `remove`, a guard flag gating the `remove`, or a `remove`
  inside a loop whose body may never run.
- Removal of a **different key** than the one written.
- Writes/removes on `temporary()` or `instance()` storage — out of scope for
  this lint.
- Correlation across function boundaries: the lint only joins `set` and
  `remove` within a single function body.
- A `set` in one monomorphized/closure context with a `remove` in another
  function.