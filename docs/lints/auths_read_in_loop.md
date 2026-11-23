# `auths_read_in_loop`

**Default Severity:** `warn`

**Target Resource:** [Host / CPU — authorization lookup and host data transfer](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags calls to `env.auths()` made inside a loop body (`for`, `while`, or `loop`).

## Why is this bad?

{% hint style="danger" %}
`env.auths()` returns the authorized-address list for the current contract invocation. The host materializes and copies the entire authorization tree into guest memory on every call, yet the set of authorizations is fixed before the contract starts executing. Calling `env.auths()` inside a loop repeatedly pays this host serialization and copying cost on every iteration for a value that never changes during the invocation.
{% endhint %}

## Example

```rust
// ❌ Bad: querying authorized addresses repeatedly inside a loop
for item in items.iter() {
    let auths = env.auths();
    process(item, &auths);
}
```

```rust
// ✅ Good: bind env.auths() once before the loop and reuse it
let auths = env.auths();
for item in items.iter() {
    process(item, &auths);
}
```

## Suggested Fix

{% hint style="success" %}
Bind `env.auths()` once before the loop to a local variable and reuse it within the loop body.
{% endhint %}

## What is not reported

- Calls to `env.auths()` outside a loop body.
- Hoisting `env.auths()` to a local variable before the loop and referencing it inside the loop.
- Calls to an unrelated method named `auths` on types other than `soroban_sdk::Env`.
- Calls inside closures defined inside a loop that are not directly invoked.
- Calls suppressed with `#[allow(auths_read_in_loop)]`.
