# `deploy_in_loop`

**Default Severity:** `warn`

**Target Resource:** [Host / CPU — contract deployment and Wasm instantiation](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags contract deployment calls (`deploy` or `deploy_v2`) made on a deployer receiver (`env.deployer()`) inside a loop body (`for`, `while`, or `loop`).

## Why is this bad?

{% hint style="danger" %}
Contract deployment and Wasm instantiation are among the most expensive host operations exposed by the Soroban SDK. The host must create a contract instance, register the storage footprint, and instantiate Wasm bytecode. Performing deployments inside a loop multiplies this overhead by the iteration count, rapidly exhausting CPU and execution budgets.
{% endhint %}

## Example

```rust
// ❌ Bad: deploying contracts repeatedly inside a loop
for wasm_hash in wasm_hashes.iter() {
    let _ = env.deployer().deploy(wasm_hash);
}
```

```rust
// ✅ Good: deploy once outside the loop and store the contract id
let contract_id = env.deployer().deploy(wasm_hash);
```

## Suggested Fix

{% hint style="success" %}
Deploy once and store the contract id, or move the deployment out of the loop. If the loop is an intentional factory pattern deploying a batch of distinct contracts by design, suppress the warning with `#[allow(deploy_in_loop)]`.
{% endhint %}

## Relationship to `unnecessary_host_function_call`

Calls to `env.deployer()` inside a loop may also be flagged by [`unnecessary_host_function_call`](unnecessary_host_function_call.md), which suggests caching the `Deployer` object outside the loop. While `unnecessary_host_function_call` identifies redundant accessor construction, `deploy_in_loop` specifically addresses the cost of the deployment itself. In factory loops where deployment per iteration is intended, allow both lints at the function or loop site:

```rust
#[allow(deploy_in_loop, unnecessary_host_function_call)]
fn factory_deploy(env: &Env, hashes: &[BytesN<32>]) {
    for hash in hashes {
        env.deployer().deploy(hash.clone());
    }
}
```

## What is not reported

- Calls to `deploy` or `deploy_v2` outside a loop body.
- Constructing `env.deployer()` without calling `deploy` or `deploy_v2` (handled by `unnecessary_host_function_call`).
- Deployments inside closures that are defined inside a loop.
- Calls suppressed with `#[allow(deploy_in_loop)]`.
