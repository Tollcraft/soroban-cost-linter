# `std_collection_in_contract`

**Default Severity:** `warn`

**Target Resource:** [Memory — wasm linear memory allocation and host-boundary conversion](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags usage of `std::collections::HashMap`, `std::collections::BTreeMap`, and
`std::vec::Vec` inside Soroban contract code (functions within a
`#[contractimpl]` block).

## Why is this bad?

{% hint style="danger" %}
Using std collections inside a Soroban contract allocates in wasm linear memory
rather than through the host. This has two costs:

1. **Binary inflation** — the allocator is compiled into the wasm binary,
   increasing the deploy cost.
2. **Host-boundary conversion** — values cannot cross the host boundary without
   an explicit conversion that the author pays for on every use.

Contracts written by developers new to Soroban reach for `HashMap` by reflex,
and the resulting cost is invisible until they compare a deployed binary against
one using `soroban_sdk::Map`. This lint makes it visible at compile time.
{% endhint %}

## Example

```rust
// ❌ Bad: std HashMap allocates in wasm linear memory
#[contractimpl]
impl MyContract {
    fn process(env: Env) {
        let mut map: HashMap<String, i32> = HashMap::new();
        map.insert("key".to_string(), 42);
    }
}
```

```rust
// ✅ Good: soroban_sdk::Map allocates through the host
#[contractimpl]
impl MyContract {
    fn process(env: Env) {
        let map: soroban_sdk::Map<soroban_sdk::Symbol, i32> = soroban_sdk::Map::new(&env);
        map.set(&soroban_sdk::symbol_short!("key"), &42);
    }
}
```

## Suggested Fix

{% hint style="success" %}
Replace std collection types with their Soroban SDK equivalents:

| Std type | Soroban equivalent |
| --- | --- |
| `std::collections::HashMap<K, V>` | `soroban_sdk::Map<K, V>` |
| `std::collections::BTreeMap<K, V>` | `soroban_sdk::Map<K, V>` |
| `std::vec::Vec<T>` | `soroban_sdk::Vec<T>` |

The Soroban SDK types allocate through the host and can cross the host boundary
without conversion.
{% endhint %}

## How contract code is identified

The lint fires inside any `impl` block that carries the `#[contractimpl]`
attribute. This is a narrow, correct boundary:

- **Inside `#[contractimpl]`** — fires.
- **Outside `#[contractimpl]`** (free functions, plain `impl` blocks, trait
  impls) — does not fire.
- **In test code** (`#[test]` functions, `#[cfg(test)]` modules) — does not
  fire, because std collections are idiomatic and correct in tests.
- **Helper functions called from `#[contractimpl]`** — does not fire, because
  the helper itself is not inside the attribute boundary. This is intentional:
  a narrow, correct lint beats a broad, noisy one.

## What is not reported

- `std::collections::HashMap`, `BTreeMap`, or `Vec` usage outside of
  `#[contractimpl]` blocks.
- `soroban_sdk::Map` or `soroban_sdk::Vec` usage (these are the correct types).
- Calls suppressed with `#[allow(std_collection_in_contract)]`.
- Non-collection std types (e.g. `String`, `Box`, `Rc`) — the lint only covers
  the three collection types listed above.
