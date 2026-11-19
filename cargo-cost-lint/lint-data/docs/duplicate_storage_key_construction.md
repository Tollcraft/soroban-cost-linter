# `duplicate_storage_key_construction`

**Default level:** `warn`

**Target Resource:** [CPU — host function dispatch and execution, Memory — host allocations](../cost_rationale.md#per-lint-resource-summary)

## What it does

This lint detects when the **same storage key expression** is constructed in two or more distinct function bodies within a crate. For example, five functions that each call `Symbol::new(env, "balance")` inline are flagged at each construction site.

## Why is this bad?

There are two problems:

1. **Wasted host calls:** Each `Symbol::new` is a host function call that crosses the Wasm boundary. Five functions constructing the same key pay the symbol-construction host call five times over.

2. **Silent typo risk:** Five independent construction sites have five independent chances to typo the key into a silent, undebuggable state bug. One `"balace"` typo splits one logical entry into two — no compiler error, no panic, just incorrect state.

The fix is the same one experienced Soroban authors apply by habit: hoist keys into constants or a key enum, construct once. This lint makes that habit enforceable.

## Example

```rust
// BAD: same key constructed in two functions
fn get_balance(env: &Env) -> i128 {
    let key = Symbol::new(env, "balance");
    let val: Option<i128> = env.storage().instance().get(&key);
    val.unwrap_or(0)
}

fn set_balance(env: &Env, amount: i128) {
    let key = Symbol::new(env, "balance");
    env.storage().instance().set(&key, &amount);
}
```

```rust
// GOOD: key constructed once via constant
const BALANCE_KEY: &str = "balance";

fn get_balance(env: &Env) -> i128 {
    let key = Symbol::new(env, BALANCE_KEY);
    let val: Option<i128> = env.storage().instance().get(&key);
    val.unwrap_or(0)
}

fn set_balance(env: &Env, amount: i128) {
    let key = Symbol::new(env, BALANCE_KEY);
    env.storage().instance().set(&key, &amount);
}
```

## Suggested Fix

Hoist the key expression to a `const` or a key enum variant. If you have many keys, consider a dedicated key enum:

```rust
enum StorageKey {
    Balance,
    Config,
    Admin,
}

impl StorageKey {
    fn symbol(&self, env: &Env) -> Symbol {
        match self {
            StorageKey::Balance => Symbol::new(env, "balance"),
            StorageKey::Config => Symbol::new(env, "config"),
            StorageKey::Admin => Symbol::new(env, "admin"),
        }
    }
}
```

## What is **not** reported

- A key constructed **once** and referenced through a constant does not fire.
- A key whose **construction differs** (different literal, runtime-derived payload) does not fire.
- Keys constructed inside different `#[cfg(test)]` modules do not fire.

## Known False Positives

- **Cross-contract key sharing:** A factory contract may construct the same key pattern as a child contract, but the key spaces are separate. Suppress with `#[allow(duplicate_storage_key_construction)]`.
- **Intentional key derivation:** Some contracts intentionally build keys from different prefixes for different access patterns. Suppress at the call site.

## Relationship to other lints

- **`storage_key_construction_in_loop`** catches the same key rebuilt on every iteration of a single loop. This lint catches the same key rebuilt across *different function bodies* — a larger-scale version of the same waste.
- **`symbol_new_for_short_literal`** catches `Symbol::new` calls with short literals that could use `symbol_short!()`. This lint catches duplicate construction regardless of the key's length.
