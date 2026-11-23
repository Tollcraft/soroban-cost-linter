# `include_bytes_in_contract`

**Default Severity:** `warn`

**Target Resource:** [Memory / RAM — Wasm bytecode size and deployment costs](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags expansions of `include_bytes!` and `include_str!` inside contract code.

## Why is this bad?

{% hint style="danger" %}
`include_bytes!` and `include_str!` splice the entire file payload into the contract's Wasm data section. The payload is paid for in Wasm bytecode size and ledger deployment fees on every deploy, even when an invocation never accesses the embedded data.
{% endhint %}

## Example

```rust
// ❌ Bad: embedding file payload directly into contract binary
const DATA: &[u8] = include_bytes!("payload.bin");
```

```rust
// ✅ Good: load payload from storage or accept it as a caller argument
fn process(env: &Env, payload: Bytes) {
    // payload provided by caller or retrieved from contract storage
}
```

## Suggested Fix

{% hint style="success" %}
Load the payload from storage or pass it as an argument.
{% endhint %}

## Relationship to `large_constant_array`

While [`large_constant_array`](large_constant_array.md) detects large inline array and repeat literals exceeding 4096 elements, `include_bytes_in_contract` specifically flags macro-included files (`include_bytes!` and `include_str!`) that embed raw file assets into the Wasm bytecode.

## What is not reported

- Other macro expansions such as `concat!`.
- Custom `macro_rules!` macros with different names.
- Array literals below the `large_constant_array` threshold.
- Calls suppressed with `#[allow(include_bytes_in_contract)]`.
