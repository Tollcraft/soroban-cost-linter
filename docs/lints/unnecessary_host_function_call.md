# `unnecessary_host_function_call`

**Default Severity:** `warn`

**Target Resource:** [CPU — host function dispatch and execution](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags calls to Soroban host functions inside a loop body when the call takes the
same inputs on every iteration.

The lint covers the host accessors reachable from `Env`:

| Accessor                        | Type(s) matched                                             |
| ------------------------------- | ----------------------------------------------------------- |
| `env.ledger()`                  | `soroban_sdk::ledger::Ledger`                                |
| `env.crypto()`                  | `soroban_sdk::crypto::Crypto`, `CryptoHazmat`, `Bls12_381`, `Bn254` |
| `env.prng()`                    | `soroban_sdk::prng::Prng`                                    |
| `env.events()`                  | `soroban_sdk::events::Events`                                |
| `env.deployer()`                | `soroban_sdk::deploy::Deployer`, `DeployerWithAddress`, `DeployerWithAsset` |
| `env.current_contract_address()`| `soroban_sdk::Env`                                           |

Storage accessors (`env.storage()`) are handled by
[`soroban_storage_in_loop`](soroban_storage_in_loop.md) instead, so they are not
reported twice.

## Why is this bad?

{% hint style="danger" %}
Calling a host function crosses the Wasm-host boundary, which incurs `DispatchHostFunction` overhead plus whatever work the function performs. Repeating this unnecessarily inside a loop adds up to **significant CPU waste**, especially when the result is constant across iterations. See the [Cost Rationale — What Dominates](../cost_rationale.md#what-dominates) for the relative cost hierarchy.
{% endhint %}

## Example

```rust
// ❌ Bad: same host call repeated every iteration
for item in items {
    let current_seq = env.ledger().sequence();
}

// ❌ Bad: the hashed input never changes
for _ in 0..n {
    let digest = env.crypto().sha256(&payload);
}
```

```rust
// ✅ Good: the argument changes every iteration, so the call is real work
for chunk in chunks.iter() {
    let digest = env.crypto().sha256(chunk);
}
```

## Suggested Fix

{% hint style="success" %}
Call the host function once before the loop, store the result in a local variable, and reference that variable inside the loop.
{% endhint %}

## What is not reported

A call is left alone when it reads anything that changes between iterations —
the loop variable, a `let` inside the loop body, or a variable mutated by the
loop. Hoisting such a call would change behaviour, so it is not a cost problem
the lint should raise.

When the mutation analysis cannot reach a verdict for a loop, every call in that
loop is left alone rather than reported on incomplete information.

Two gaps remain, and both make the lint report a call it could have skipped:

- Bindings and mutations inside a closure body nested in the loop are not seen.
- Mutation through a raw pointer or through interior mutability (`Cell`,
  `RefCell`) is not tracked.

## Deliberately not covered

- `env.invoke_contract()`, `env.try_invoke_contract()` and
  `env.authorize_as_current_contract()` — invoking or authorizing per iteration
  is what the loop is for, and the cost is inherent rather than redundant.
- Calls whose per-iteration repetition is intentional, most commonly
  `env.prng()` generators and `env.events().publish()` with constant arguments,
  are still reported: they are metered host calls with unchanged inputs, and the
  lint cannot tell intent. Use `#[allow(unnecessary_host_function_call)]` on
  those call sites.
