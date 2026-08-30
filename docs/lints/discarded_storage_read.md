# `discarded_storage_read`

**Default Severity:** `warn`

## What it does

Detects reads from storage whose result is never used (e.g., bound to `_`, evaluated purely as a statement, or bound and never subsequently referenced).

## Why is this bad?

{% hint style="danger" }
Storage reads are among the most expensive operations available to a Soroban contract. Performing a read whose result is discarded is pure waste with no behavioral purpose, unnecessarily driving up resource metering and network fees.
{% endhint }

## Example

```rust
// ❌ Bad: storage read result is discarded
let _ = env.storage().instance().get::<u32, i32>(&key);
env.storage().persistent().get::<u32, i32>(&key); //~ WARNING
```

## Suggested Fix

{% hint style="success" }
Delete the storage read entirely if the state is not needed.
{% endhint }

```rust
// ✅ Good: read omitted
```

## Scope

- Flags `get` and `has` calls on `Instance`, `Persistent`, and `Temporary` storage where the returned value is unused.
- Deliberate existence checks where the result *is* consumed (e.g. `if env.storage().persistent().has(&key)`, or matching on an `Option` read) are not flagged.
