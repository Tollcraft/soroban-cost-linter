# `unwrap_on_storage_get`

**Default Severity:** `warn`

## What it does

Detects `.unwrap()` or `.expect()` called directly on a Soroban storage read (`get` on the `Storage`, `Instance`, `Persistent`, or `Temporary` accessors). A storage `get` returns an `Option` precisely because the key may never have been written, or its entry may have expired — unwrapping turns that expected case into a contract trap.

## Why is this bad?

{% hint style="danger" %}
Soroban meters every host call a contract makes, and work performed before a trap is still charged. A contract that reads three storage entries and panics on the fourth has paid for four reads and delivered nothing: the transaction fails, the user gets no useful error, and the fees are gone.

This is why the pattern belongs in a cost linter rather than only a correctness one — the failure mode wastes metered work that has already been billed.
{% endhint %}

## Example

```rust
// ❌ Bad: panics on the first caller who hits a missing or expired key
let value: i32 = env.storage().persistent().get(&key).unwrap();
```

## Suggested Fix

{% hint style="success" %}
Handle the `None` case explicitly — with `unwrap_or`, `unwrap_or_else`, or an early return carrying a proper error (`panic_with_error!` + `#[contracterror]`). This turns a wasted invocation into a cheap one: the failure path costs a single early return instead of everything metered before it.
{% endhint %}

```rust
// ✅ Good: explicit error path
let value = match env.storage().persistent().get::<_, i32>(&key) {
    Some(v) => v,
    None => return Err(Error::KeyNotFound),
};

// ✅ Also good: a default instead of a trap
let value: i32 = env.storage().persistent().get(&key).unwrap_or(0);
```

## Scope

- Fires on `.unwrap()` and `.expect()` whose receiver is directly a storage `get` or `try_get`, written either as a method call (`env.storage().instance().get(&k).unwrap()`) or as a UFCS call (`Instance::get(&storage, &k).unwrap()`).
- Only the four Soroban storage types are matched: `soroban_sdk::storage::Storage`, `Instance`, `Persistent` and `Temporary`. A `get` on any other type, such as `soroban_sdk::Map` or a plain `Option`, is not reported.
- References are peeled before the type is checked, so a `get` through a `&Instance` or `&Persistent` binding is matched the same as one on the value itself.
- Only the `get` and `try_get` accessors are matched. `has` returns a `bool` rather than an `Option`, so there is nothing to unwrap and it never matches.
- Does not fire when the returned `Option` is matched, handled with `unwrap_or`/`unwrap_or_else`, or otherwise consumed without panicking.
- Does not fire on `unwrap` anywhere other than directly on a storage read (e.g. on SDK collections or plain `Option`s).
- Does not fire under `#[cfg(test)]` or inside test modules. A panic in a test is a failed assertion, not a trapped invocation: no metered budget is spent and no caller is charged, so the cost this lint exists to catch does not arise, and unwrap in tests is idiomatic.

## Known false positives

A key the contract has genuinely just written earlier in the same invocation cannot miss, yet the unwrap still fires because the lint does not track writes across statements. See [Handling False Positives](../false_positives.md#unwrap_on_storage_get).
