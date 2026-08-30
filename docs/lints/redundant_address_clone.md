# `redundant_address_clone`

**Default Severity:** `warn`

**Target Resource:** [CPU — memory allocation, copy, and host object dispatch](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects unnecessary `.clone()` calls on the Soroban `Address` object. The lint
fires when a `.clone()` is invoked on a local `Address` binding whose original
value is not used afterward and could instead be moved or passed by reference.

## Why is this bad

{% hint style="danger" %}
`Address` is a host-side handle in the Soroban SDK. Cloning an `Address`
duplicates host-side handle state and is a metered operation; each clone
consumes CPU and memory budget on the network.
{% endhint %}

When clones are taken reflexively (for example, before calling `require_auth`,
passing into token clients, building storage keys, or emitting events) they can
outnumber the real work performed by the function and inflate transaction
costs.

## Cost impact

Every `Address::clone()` duplicates host-side handle structures, causing
additional CPU instructions and possible memory traffic. The per-clone cost is
small but can be significant in aggregate in hot paths.

Measured guidance and benchmark references are provided in the project's
`cost_benchmarks` crate; see `cost_benchmarks/` for microbenchmarks related to
host-handle cloning patterns.

## How to reproduce

Run the UI test suite for lints or compile a contract containing redundant
`Address::clone()` sites under this crate's test harness. The UI fixture
`soroban_cost_lints/ui/redundant_address_clone.rs` demonstrates triggering and
non-triggering cases.

## Example

```rust
// ❌ Bad: original `addr` is not used after clone
fn bad(addr: soroban_sdk::Address) {
    let _c = addr.clone();
}

// ✅ Good: move the value instead of cloning
fn good(addr: soroban_sdk::Address) {
    takes_addr(addr);
}
```

## Known False Positives (Not Flagged)

The lint deliberately takes a conservative posture to avoid false positives in
the most common legitimate cases:

1. **`&Address` receiver** — Cloning through a reference produces an owned
   `Address` from a borrowed one; this is a valid pattern to satisfy ownership
   / borrow-checker requirements and is not flagged.
2. **Original binding reused after clone** — If the same `Address` binding is
   used again after the `.clone()` call, both the original and the clone are
   live and the lint does not fire.
3. **Non-local receiver** — When the receiver is not a simple local binding
   (e.g. a struct field access), the analysis is conservative and skips the
   site.

If this lint flags a site you intentionally want to keep, suppress it with
`#[allow(redundant_address_clone)]`.

## Suggested Fix

{% hint style="success" %}
Pass `Address` by value (moving it) or by reference instead of calling
`.clone()`. This eliminates unnecessary host-side duplication.
{% endhint %}
