# `extend_ttl_in_loop`

**Default Severity:** `warn`

**Target Resource:** [Storage — ledger space rent (TTL extension)](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags a call to `extend_ttl` on instance, persistent, or temporary storage
(`env.storage().instance().extend_ttl(...)`,
`env.storage().persistent().extend_ttl(&key, ...)`,
`env.storage().temporary().extend_ttl(&key, ...)`) when the call site sits
directly inside a loop body (`for`, `while`, or `loop`).

## Why is this bad?

{% hint style="danger" %}
`extend_ttl` is a metered host call that *also* writes ledger state — it is
not a read-only query. Extending an entry's TTL incurs a rent payment (see
[Cost Rationale — Ledger Space Rent](../cost_rationale.md#6-ledger-space-rent)),
priced dynamically based on ledger size. Issuing one `extend_ttl` call per
iteration — the natural shape when refreshing the TTL of a set of entries —
multiplies both the host-call dispatch cost and the rent cost by the
iteration count, exactly the same structural problem
[`soroban_storage_in_loop`](soroban_storage_in_loop.md) flags for `get` /
`has` / `set`.
{% endhint %}

## Example

```rust
// ❌ Bad: one extend_ttl host call (and rent payment) per iteration
for key in keys.iter() {
    env.storage().persistent().extend_ttl(&key, 100, 1000);
}
```

```rust
// ✅ Good: collect the keys first, then extend once per key outside the
// loop (or, if the SDK/contract design allows it, size the threshold
// generously enough that a single extension up front covers the whole
// batch instead of refreshing per-entry per-iteration)
let keys: Vec<_> = collect_keys();
for key in keys.iter() {
    env.storage().persistent().extend_ttl(&key, 100, 1000);
}
// (the call above still runs once per key, but only after the set of
// keys needing extension has been determined — see "Suggested Fix" for
// batching the extension itself where the storage accessor allows it)
```

## Suggested Fix

{% hint style="success" %}
Batch the extension instead of refreshing per-entry per-iteration:

- Collect the keys/entries that need a TTL refresh first (in memory), then
  issue the `extend_ttl` calls after the loop that determines which entries
  need it, rather than intermixing the decision loop with the host call.
- If the entries share one accessor (e.g. multiple keys under
  `Persistent`), look for opportunities to extend fewer, larger entries
  instead of many small ones — fewer entries means fewer `extend_ttl` calls
  are needed in the first place.
- Extend once with a threshold sized generously enough to cover the whole
  batch's expected lifetime, instead of issuing a tight, frequently-refreshed
  extension per entry per iteration.
{% endhint %}

## What is not reported

- `extend_ttl` calls outside of a loop body.
- Calls on receivers that are not `soroban_sdk::storage::{Instance,
  Persistent, Temporary}` (or `Storage` itself).
- Calls suppressed with `#[allow(extend_ttl_in_loop)]`.
- Whether the TTL threshold/extend-to values passed to `extend_ttl` are
  sensible — this lint only looks at the structural in-loop pattern, not
  the argument values.

## Relationship to `soroban_storage_in_loop`

[`soroban_storage_in_loop`](soroban_storage_in_loop.md) has two detection
arms. Its **direct** in-loop check only treats a method call as a storage
access when the method name is `get`, `has`, or `set` — `extend_ttl` is not
in that allowlist, so the direct arm never fires on it. This lint owns the
direct in-loop `extend_ttl` diagnostic instead, and there is no overlap
between the two: the same call is never reported twice.

`soroban_storage_in_loop` also has a second, inter-procedural arm that walks
into callees reachable from a loop and flags *any* method call on a
storage-typed receiver found inside them, without a method-name filter. A
callee containing `env.storage().instance().extend_ttl(...)` and invoked
from a loop could in principle be caught by that arm — this is a distinct,
already-existing code path unrelated to the direct check this lint adds, and
is out of scope for this lint, which only targets the syntactic, direct
in-loop call (mirroring how [`map_insert_in_loop`](map_insert_in_loop.md)
and [`bytes_append_in_loop`](bytes_append_in_loop.md) are scoped).
