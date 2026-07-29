# `require_auth_in_loop`

**Default Severity:** `warn`

**Target Resource:** [CPU — host function calls and authentication dispatch](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects calls to `Address::require_auth()` or `Address::require_auth_for_args()` whose receiver type is `soroban_sdk::Address` and that appear inside a loop body (`for`, `while`, or `loop`).

## Why is this bad?

{% hint style="danger" %}
`require_auth` (and its argument-taking variant) runs a host-side authorization check that is comparatively expensive — it traverses the auth tree, verifies signatures, and accounts for the matching `require_auth` records the contract previously installed. Re-running it on every iteration of a loop multiplies that work by the iteration count, even when the same address is authorized in every iteration.
{% endhint %}

In most real contracts the set of addresses that need authorization is already known before the loop is entered. In that common case the fix is to hoist the call(s) out of the loop:

```rust
// ❌ Bad: authorize the same address once per iteration
for item in items {
    addr.require_auth();
    transfer(item);
}
```

## Suggested Fix

{% hint style="success" %}
Collect the distinct addresses first and authorize each one once before the loop body runs:
{% endhint %}

```rust
// ✅ Good: authorize each address once, then loop
addr.require_auth();
for item in items {
    transfer(item);
}
```

If the loop genuinely iterates over **different** addresses each iteration, that pattern is sometimes legitimate. The lint still warns (so the call is visible during review), but you can suppress it at that call site with `#[allow(require_auth_in_loop)]` and/or collect unique addresses first and call `require_auth` once per distinct value.
