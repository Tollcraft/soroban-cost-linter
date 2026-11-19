# `redundant_address_clone`

**Default Severity:** `warn`

**Target Resource:** [CPU — host object dispatch and memory operations](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects `addr.clone()` on a `soroban_sdk::Address` when the original binding
is never read again.

## Why is this bad?

{% hint style="danger" %}
`Address` is a handle to a host-side object: cloning it crosses the host
boundary to duplicate the host object reference. When the original is never
used afterwards — the clone is passed to a function, stored in a block, or
immediately shadows the binding — that host work buys nothing. See the
[Cost Rationale — CPU](../cost_rationale.md#1-cpu-instructions).
{% endhint %}

## Example

```rust
// ❌ Bad: `addr` is not used after the clone
let copy = addr.clone();

// ✅ Good: move the original
let copy = addr;
```

## Known False Positives (Not Flagged)

The lint stays silent when the clone may be genuinely needed:

1. **`&Address` receiver** — cloning through a reference produces the owned
   `Address` the caller asked for.
2. **Original binding reused after the clone** — both values are live.
3. **Non-local receiver** — a field access such as `record.addr.clone()`, where
   last use cannot be decided from the expression alone.

```rust
// ✅ Not flagged: &Address
fn use_ref(addr: &Address) {
    let owned = addr.clone();
}

// ✅ Not flagged: original still used
fn both_live(addr: Address) {
    let copy = addr.clone();
    use_addr(addr);
}
```

## Suggested Fix

Move the `Address` instead of cloning it. If both values really are needed,
check whether the second use can take a reference (`&addr`) instead.
