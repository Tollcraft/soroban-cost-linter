# `persistent_read_without_ttl_extension`

**Default Severity:** `warn`

**Target Resource:** [Entry Lifecycle — TTL extension cost cliff](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags reads (`get` or `has`) from `env.storage().persistent()` in functions
where no `extend_ttl` call is made on the same storage type.

Reads from `instance()` or `temporary()` storage are not reported — those
storage types have different lifecycle semantics and are not subject to the
same archival-cost cliff.

## Why is this bad?

{% hint style="danger" %}
Persistent storage entries in Soroban have a finite Time-to-Live (TTL). When
an entry's TTL expires, the entry is **archived** and must be restored on the
next access — a restoration that costs roughly **3× a normal read** in CPU,
I/O, and ledger entry write resources.

Reading from persistent storage without extending the TTL means the *next*
contract invocation that touches that key will pay the archival cost.
Repeating this pattern across many entries or many ledgers can cause
unpredictable fee spikes.

See the [Cost Rationale — Entry Lifecycle](../cost_rationale.md#4-entry-lifecycle-ttl-and-archival) for details.
{% endhint %}

## Example

```rust
use soroban_sdk::{Env, storage::Persistent};

// ❌ Bad: reads persistent storage without extending TTL
fn read_without_ttl(env: Env) {
    let _val: Option<i32> = env.storage().persistent().get(&1);
}
```

```rust
use soroban_sdk::{Env, storage::Persistent};

// ✅ Good: TTL is extended after the read
fn read_with_ttl(env: Env) {
    env.storage().persistent().extend_ttl(&1, &());
    let _val: Option<i32> = env.storage().persistent().get(&1);
}
```

## Suggested Fix

{% hint style="success" %}
After reading from persistent storage, call `extend_ttl` on the same key with
an appropriate threshold to keep the entry alive for future invocations.
{% endhint %}

## Known Limitations

- The lint uses a conservative whole-function analysis: if **any**
  `extend_ttl` call exists on persistent storage anywhere in the function,
  all persistent-read diagnostics are suppressed. This avoids false positives
  when the developer is clearly TTL-aware, even if a particular read uses a
  different key than the `extend_ttl` call.
- Reads guarded by a `has` check that handles the archived case are still
  reported.
