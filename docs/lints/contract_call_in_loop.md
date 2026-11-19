# `contract_call_in_loop`

**Default Severity:** `warn`

**Target Resource:** [CPU — host function dispatch and cross-contract VM instantiation](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags cross-contract invocations made through `env.invoke_contract(...)` when the
call site sits inside a loop body (`for`, `while`, or `loop`).

## Why is this bad?

{% hint style="danger" %}
A cross-contract call is one of the most expensive operations a Soroban contract can perform: the host has to instantiate a new VM context for the callee, re-charge its own dispatch and instantiation overhead (`DispatchHostFunction`), and add whatever footprint the callee itself has. Repeating this per loop iteration multiplies that overhead by the iteration count, and is almost always a sign of a missing batch endpoint on the callee, or a call whose result could be computed once and reused. See the [Cost Rationale — What Dominates](../cost_rationale.md#what-dominates) for the relative cost hierarchy.
{% endhint %}

## Example

```rust
// ❌ Bad: one cross-contract call per iteration
for item in items.iter() {
    let _: i128 = env.invoke_contract(&token_address, &symbol_short!("balance"), (item,).into_val(&env));
}
```

```rust
// ✅ Good: a single batched call to the callee
let balances: Vec<i128> = env.invoke_contract(
    &token_address,
    &symbol_short!("balances"),
    (items.clone(),).into_val(&env),
);
```

## Suggested Fix

{% hint style="success" %}
Prefer a bulk endpoint on the callee contract that accepts the whole batch of inputs and returns all results in one call. If no such endpoint exists, consider adding one to the callee. When the call is invariant across iterations (the arguments never change), hoist it out of the loop and reuse the result instead.
{% endhint %}

## What is not reported

- Calls to `invoke_contract` outside of a loop body.
- Calls suppressed with `#[allow(contract_call_in_loop)]`.

## Deliberately not covered

This lint starts from the directly detectable case and leaves two related patterns
as documented follow-ups:

- **`env.try_invoke_contract(...)`** — the fallible counterpart of `invoke_contract`
  is not yet matched. It shares the same cost profile and is a natural extension.
- **Generated contract clients** — types produced by `contractimport!` /
  `contractclient!` (conventionally `*Client` structs) wrap `invoke_contract`
  internally, so calling `token_client.transfer(...)` inside a loop has the same
  cost profile as calling `env.invoke_contract(...)` directly, but is not
  currently detected. Resolving this requires recognizing generated client types
  by structural convention rather than a fixed path, which is left for a
  follow-up once the type-resolution approach is settled.
