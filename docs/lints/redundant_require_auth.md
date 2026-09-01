# redundant_require_auth

| Property | Value |
| --- | --- |
| Default severity | `warn` |
| Category | Authorization / Compute |

## What it catches

Calling `require_auth` (or `require_auth_for_args`) more than once on the same `Address` within a single function body.

## Why it matters

`require_auth` walks the authorization tree and verifies signatures for the given address. Calling it twice for the same address in the same invocation proves nothing new, but costs the full signature-verification and authorization work a second time — crossing the host-function boundary into the VM twice for the same result.

## Triggering example

```rust
pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
    from.require_auth();
    // ... work ...
    from.require_auth(); // warn: redundant_require_auth
    // transfer ...
}
```

## Recommended rewrite

Call `require_auth` exactly once per address, before performing the sensitive operation:

```rust
pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
    from.require_auth();
    // ... work, then transfer ...
}
```

## When NOT to suppress

Do **not** remove a *second* `require_auth` on a *different* address, or suppress this lint as a way to skip authorization. This lint only flags the same address verified more than once; it never suggests removing authorization.

## When to suppress

If you deliberately re-verify authorization after a cross-contract call that may have shifted the authorization context, suppress the lint at that call site:

```rust
#[allow(redundant_require_auth)]
from.require_auth();
```
