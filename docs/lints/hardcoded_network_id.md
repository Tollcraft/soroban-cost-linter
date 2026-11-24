# `hardcoded_network_id`

**Default Severity:** `warn`

**Target Resource:** [Compute / Security — Host function dispatch and failed transaction execution cost](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects compile-time literal 32-byte network IDs (`[u8; 32]`, `&[u8; 32]`, byte strings) or network passphrases compared against `env.ledger().network_id()` or embedded in contract code.

## Why is this bad?

Embedding a hardcoded network ID or network passphrase in contract code compile-time pins the contract to a single network (such as Testnet or a local standalone sandbox).

On Soroban, network identity is dynamically provided by the host environment via `env.ledger().network_id()`. When contract logic checks or verifies against a hardcoded network identifier, deploying the same contract to a different network (e.g. from Testnet to Mainnet) causes transactions to fail during execution, burning metered CPU instructions and transaction fees for failed invocations. In addition, hardcoding network passphrases inflates the contract WASM binary size, incurring higher deployment fees under the Soroban host WASM size meter.

## Example

```rust
use soroban_sdk::Env;

const TESTNET_ID: [u8; 32] = [0u8; 32];

// ❌ Bad: hardcoded network ID comparison
fn verify_network(env: &Env) {
    if env.ledger().network_id() == TESTNET_ID {
        // ...
    }
}
```

```rust
use soroban_sdk::Env;

// ✅ Good: retrieve the network ID dynamically from the ledger context
fn process(env: &Env) {
    let current_net = env.ledger().network_id();
    // Use or forward current_net without comparing to compile-time network literals
}
```

## Relationship with `ledger_context_read_in_loop`

`ledger_context_read_in_loop` flags calling `env.ledger().network_id()` inside a loop because ledger context is invariant across iterations. `hardcoded_network_id` specifically flags the *literal* network identifier being compared or initialized, so the two lints do not duplicate each other.

## When to suppress

If a test contract or mock specifically validates a mock environment against a known fixed network ID, suppress at the call site:

```rust
#[allow(hardcoded_network_id)]
if env.ledger().network_id() == [0u8; 32] {
    // ...
}
```
