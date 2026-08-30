# `token_transfer_in_loop`

**Default Severity:** `warn`

**Target Resource:** [CPU — host function dispatch and cross-contract VM instantiation](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags `transfer` and `transfer_from` calls on a Soroban contract client type when
the call site sits inside a loop body (`for`, `while`, or `loop`).

## Why is this bad?

{% hint style="danger" %}
A token transfer is a cross-contract call plus at least two storage writes, and it
is one of the most expensive single operations a Soroban contract can perform.
Performing one per iteration — an airdrop loop, a batch payout, a fee distribution
— multiplies that cost by a caller-influenced factor and is a common way for a
contract to become unusable at scale after testing fine with three recipients.

See the [Cost Rationale — What Dominates](../cost_rationale.md#what-dominates) for
the relative cost hierarchy.
{% endhint %}

## Example

```rust
// ❌ Bad: one token transfer per iteration
for recipient in recipients.iter() {
    token_client.transfer(&owner, recipient, &amount);
}
```

```rust
// ✅ Good: claim pattern — recipients pull instead of the contract pushing
// A separate `claim(amount)` endpoint lets each recipient initiate their own
// transfer, avoiding the per-iteration cross-contract call.
fn claim(env: Env, amount: i128) {
    let caller = env.invoker();
    token_client.transfer(&owner, &caller, &amount);
}
```

## Suggested Fix

{% hint style="success" %}
Prefer a **claim pattern** where recipients pull tokens by calling a
`claim(amount)` endpoint on the contract, rather than having the contract push
tokens to every recipient in a loop. Each claim is a single, bounded-cost
cross-contract call driven by the recipient, not a loop whose iteration count is
controlled by the caller.

If a push model is required, add a **bulk transfer** endpoint on the token
contract that accepts the full list of `(recipient, amount)` pairs and executes
them in a single host call, eliminating the per-iteration dispatch overhead.
{% endhint %}

## Relationship to `contract_call_in_loop`

Both `contract_call_in_loop` and `token_transfer_in_loop` may fire on the same
code. `contract_call_in_loop` catches the general pattern of any
`env.invoke_contract` inside a loop. `token_transfer_in_loop` additionally
matches `transfer` and `transfer_from` calls on generated contract clients
(types produced by `contractimport!` / `contractclient!`, conventionally
`*Client` structs), which wrap `invoke_contract` internally.

When both fire, `token_transfer_in_loop` provides the more actionable fix —
the claim pattern or bulk transfer — while `contract_call_in_loop` provides
the general batching advice. You can suppress the more generic lint with
`#[allow(contract_call_in_loop)]` if you prefer to keep only the specific one.

## What is not reported

- `transfer` or `transfer_from` calls outside of a loop body.
- Calls to `env.invoke_contract(...)` directly (covered by
  `contract_call_in_loop`).
- Calls suppressed with `#[allow(token_transfer_in_loop)]`.
- Non-ADT receivers (primitives, references to non-struct types) — the lint
  requires the receiver to be an ADT whose definition path does not match any
  known `soroban_sdk` type.
