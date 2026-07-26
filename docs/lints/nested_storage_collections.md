# `nested_storage_collections`

**Default Severity:** `warn`

**Target Resource:** [CPU — host-object serialization, Storage — ledger I/O bytes](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags `env.storage()....set(key, value)` calls where the key or the value type
is a Soroban `Map` or `Vec` that itself contains another `Map` or `Vec` as one
of its generic type arguments — for example `Map<Symbol, Map<u32, i128>>` or
`Map<Symbol, Vec<i128>>`.

## Why is this bad?

{% hint style="danger" %}
Soroban storage operations are billed on host-object serialization overhead, not just
raw byte size. A storage value is a single host object graph: reading or
writing it deserializes/re-serializes the *entire* structure, however deep it
nests. Storing a `Map<Symbol, Map<u32, i128>>` as one storage entry means that
updating a single inner `i128` requires the host to deserialize the whole
outer map, every inner map it contains, mutate one value, and re-serialize
everything back — CPU and memory cost that scales with the total size of the
nested structure rather than with the single value that actually changed. See
[Cost Rationale — Storage](../cost_rationale.md#3-storage-ledger-entry-accesses-and-ledger-io)
for why storage operations dominate the fee.
{% endhint %}

## Example

```rust
// ❌ Bad: updating one balance re-serializes every account's balance history
let balances: Map<Symbol, Map<u32, i128>> = Map::new(&env);
env.storage().instance().set(&user, &balances);
```

```rust
// ✅ Good: a compound key flattens the structure to one value per entry
let key = (user.clone(), epoch);
env.storage().instance().set(&key, &balance);
```

## Suggested Fix

{% hint style="success" %}
Flatten the data structure by using a compound key — e.g. a tuple like
`(Symbol, u32)` — as the storage key instead of nesting a second collection
inside the value (or key). Each entry then becomes its own independent
storage object: reading or writing one no longer touches any of the others.
{% endhint %}

## What is not reported

- A single `Map` or `Vec` whose generic arguments are all scalar or otherwise
  non-collection types (e.g. `Map<Symbol, i128>`, `Vec<i128>`) — one level of
  collection is the normal, expected shape of Soroban storage.
- `.set()` calls whose receiver is not a Soroban storage accessor
  (`env.storage().instance()` / `.persistent()` / `.temporary()`).
- Calls suppressed with `#[allow(nested_storage_collections)]`.

## Deliberately not covered

This lint inspects the immediate generic arguments of the type passed to
`.set()`. Two related patterns are left as documented follow-ups:

- **Nesting behind a type alias or wrapper struct** — a user-defined struct
  that internally holds a `Map<K, Map<...>>` field is not unwrapped; only the
  type argument passed directly to `.set()` is inspected.
- **`Bytes` containing serialized nested data** — a `Map` manually serialized
  into a `Bytes` blob before storage has the same re-serialization cost, but
  is not statically distinguishable from any other `Bytes` value.
