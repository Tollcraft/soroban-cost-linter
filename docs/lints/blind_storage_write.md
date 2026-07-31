# `blind_storage_write`

**Default Severity:** `warn`

**Category:** Storage Operations

## What it does

Detects a storage write (`.set()`) on a Soroban storage bucket (`Instance`, `Persistent`, or `Temporary`) when the same key has not been read (via `.get()`, `.try_get()`, `.has()`, `.remove()`, or `.update()`) anywhere in the same function.

A HIR-level walk of the function body tracks which `(bucket, key)` pairs have been observed by a non-write call, and flags each `.set()` whose key is unknown.

## Why is this bad?

{% hint style="danger" %}
Soroban storage writes are **the most expensive ledger operation**. A blind write — one that has not been preceded by a read on the same key — risks two failure modes that cost real money:

1. **Silent overwrite.** If the contract is the only writer, an unexamined `.set()` can overwrite existing data (counter increments, accumulated state, balances) without the contract noticing. The state change is committed; nothing the user or the contract learns afterwards will reveal the data loss.
2. **Key collisions.** Without a prior read, the contract cannot distinguish between "fresh key" and "reuse of an already-populated key by another path". A name collision is a contract bug that is **structurally undetectable from tests alone** unless the test seeds both writes.

A read-before-write pattern makes both failure modes surface at compile-time as intentional choices, not silent regressions.
{% endhint %}

## Example

```rust
// ❌ Bad: a storage write with no preceding read of the same key
fn record_total(env: Env, total: u32) {
    env.storage()
        .persistent()
        .set(&"total", &total);
}
```

## Suggested Fix

{% hint style="success" %}
Read the key first with `.get()`, `.has()`, `.try_get()`, `.remove()`, or `.update()`, and only write when the contract has intentionally inspected the existing state.
{% endhint %}

```rust
// ✅ Good: explicitly read the existing value before deciding to write
fn record_total(env: Env, total: u32) {
    let prior: Option<u32> = env.storage().persistent().get(&"total");
    if prior.is_some() {
        env.storage()
            .persistent()
            .set(&"total", &total);
    }
}
```

## Non-Flagged Cases

The lint does **not** trigger when:

- A `.get()`, `.try_get()`, `.has()`, `.remove()`, or `.update()` call on the same storage bucket and with the same key appears anywhere in the same function body.
- The key expression cannot be safely compared (complex expressions whose source text we cannot match literally — we conservatively assume a read occurred).
- The write targets a different storage bucket than the read (e.g., a read on `Instance` does **not** authorise a write on `Persistent` for the same textual key).
- The function is annotated with `#[allow(blind_storage_write)]`.

## HIR-Level Limitations

The lint performs a single HIR-level walk of each function body, so reads that happen in a **different** function — for example, a read performed inside a helper that the current function calls — are **not** tracked. If your contract relies on a helper to check the key first, suppress the lint at the call site with `#[allow(blind_storage_write)]` (or use a `budget.toml` allow). Macro-expanded reads and reads inside closures invoked from the function are likewise not visible to this lint.

## Severity & Suppression

Default severity is `warn`. Adjust per-workspace via `budget.toml`:

```toml
[lints]
blind_storage_write = "deny"   # or "allow", "warn"
```

Suppress a single function with the standard Rust attribute:

```rust
#[allow(blind_storage_write)]
fn initialise_default_state(env: Env) {
    env.storage().instance().set(&"total", &0u32);
}
```