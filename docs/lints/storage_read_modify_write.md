# `storage_read_modify_write`

**Default Severity:** `warn`

## What it does

Detects when the same storage key is read, modified, and written back more than once in a single function body — i.e., two or more complete read-modify-write cycles on the same key without an intervening call that could touch storage.

## Why is this bad?

{% hint style="danger" %}
Each read-modify-write cycle performs a full metered storage read **and** a full metered storage write. When two helper functions each independently fetch, adjust, and store the same entry, the second cycle is entirely redundant — the value never left the host between the two reads. On a balance update touched by three helpers, that is six storage operations where two would do.
{% endhint %}

## Example

```rust
// ❌ Bad: two complete read-modify-write cycles on key "balance"
let val: Option<i32> = env.storage().instance().get(&"balance");
env.storage().instance().set(&"balance", &(val.unwrap_or(0) + 10));

let val2: Option<i32> = env.storage().instance().get(&"balance");
env.storage().instance().set(&"balance", &(val2.unwrap_or(0) + 20)); // redundant cycle!
```

## Suggested Fix

{% hint style="success %}
Read the value once, accumulate all modifications, then write once at the end.
{% endhint %}

```rust
// ✅ Good: single read, accumulate, single write
let mut balance = env.storage().instance().get::<_, i32>(&"balance").unwrap_or(0);
balance += 10;
balance += 20;
env.storage().instance().set(&"balance", &balance);
```

## Scope

- Fires when there are **two or more** complete read-modify-write cycles (get then set) on the same storage key within one function body.
- A **single** cycle (read once, write once) does not fire — it is the correct pattern.
- Cycles separated by a **function or method call** that could itself touch storage are not flagged, because the call may have legitimately modified the key. This is conservative: we reset all tracking when we encounter a call we cannot prove storage-free.
- **Key equality** is determined by source-text comparison of the key expression within the same storage namespace (`instance`, `persistent`, or `temporary`). Two `Symbol::new(env, "balance")` calls are considered the same key; two enum keys with different payloads are not.
- Only tracks storage access at the **top level** of statements within the same function body. Nested closures and cross-function analysis are out of scope.

## Known False Positives

- **Cross-function helper patterns:** When two helper functions each perform a read-modify-write on the same key and are called sequentially, the lint correctly fires on the second call. In some cases this is intentional (e.g., separate business-logic modules). Suppress with `#[allow(storage_read_modify_write)]`.
- **Complex control flow with branching reads:** If a key is read through different code paths (e.g., inside an `if`/`else`), the lint may reset tracking conservatively and miss or flag a cycle depending on the path taken.
