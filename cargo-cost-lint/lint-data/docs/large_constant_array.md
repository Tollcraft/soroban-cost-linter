# `large_constant_array`

**Default Severity:** `warn`

**Target Resource:** [Memory — contract storage footprint](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags large constant array **literals** in contract code whose element count
exceeds a specific size threshold (currently **4 096 elements**).

An array literal spells every element out — `[0, 1, 2, …]` — so all of them
end up in the contract's data section and the deployed binary grows with the
array. That is the shape this lint is about.

A **repeated fill** — `[0u8; 8192]` — is *not* reported. The compiler lowers
it to a zeroed region instead of embedding it, so it adds nothing to the
binary. The same holds for a fill that repeats a value only known at run
time.

## Why is this bad?

{% hint style="danger" %}
Embedding a large constant array directly in the contract source (e.g. a table
of 5 000 literal bytes) increases the final WebAssembly (WASM) binary size
linearly with the array length. Soroban heavily restricts and meters the size
of deployed contracts. Large binaries not only cost more to deploy but also
cost more gas to load into memory upon every invocation. Data this large is
often better managed dynamically using persistent storage or `Bytes`.
{% endhint %}

## Example

```rust
// ❌ Bad: every element is embedded in the contract's data section
const LARGE_DATA: [u8; 5000] = [0, 1, 2, 3, /* … */];

fn uses_data() {
    let buf = [0, 1, 2, 3, /* … */]; // Also bad
}
```

```rust
// ✅ Good: keeping the contract small and managing large data dynamically
fn uses_data(env: &Env) {
    let buf = env.storage().persistent().get(&symbol_short!("Data")).unwrap();
}
```

```rust
// ✅ Good: a zero-filled scratch buffer embeds nothing in the binary
fn scratch() -> [u8; 8192] {
    [0u8; 8192]
}
```

## Suggested Fix

{% hint style="success" }
Move large, static or semi-static datasets into Soroban persistent storage
or host-managed `Bytes`. Instead of bundling data in the Wasm, initialize 
it via an admin endpoint after deployment and read it dynamically, or accept 
the data as an invocation argument.
{% endhint %}
