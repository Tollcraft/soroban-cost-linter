# ledger_context_read_in_loop

| Property | Value |
| --- | --- |
| Default severity | `warn` |
| Category | CPU/Compute |

## What it catches

Reading a ledger context value (`sequence`, `timestamp`, `network_id`, `protocol_version`) inside a loop body.

## Why it matters

Ledger context values are **invariant during a single contract invocation**. The ledger does not advance while a contract executes, so `env.ledger().sequence()` returns the same value on every call within one invocation. Reading it inside a loop performs repeated host calls for a value that cannot change.

While this is less expensive than storage operations, it still burns CPU budget for no behavioral benefit. Hoisting the read outside the loop and reusing the result is free and makes the invariance explicit.

## Triggering example

```rust
fn process_items(env: Env, items: Vec<u32>) {
    for item in items.iter() {
        let seq = env.ledger().sequence(); // warn: ledger_context_read_in_loop
        // use seq and item...
    }
}
```

## Recommended rewrite

```rust
fn process_items(env: Env, items: Vec<u32>) {
    let seq = env.ledger().sequence(); // read once, outside the loop
    for item in items.iter() {
        // use seq and item...
    }
}
```

## Covered accessors

- `env.ledger().sequence()` → `u32`
- `env.ledger().timestamp()` → `u64`
- `env.ledger().network_id()` → `BytesN<32>`
- `env.ledger().protocol_version()` → `u32` (deprecated)

## Relationship with `host_in_loop`

The [`host_in_loop`](host_in_loop.md) lint flags **any** use of a host object inside a loop. `ledger_context_read_in_loop` is a more specific lint that provides the **invariance explanation** — it tells you *why* the read is wasteful (the value cannot change during this invocation), not just that it happens inside a loop.

A `env.ledger().sequence()` call inside a loop may trigger **both** lints. The more specific `ledger_context_read_in_loop` message is the actionable one: hoist the read.

## When to suppress

If your contract intentionally reads the ledger on each iteration for clarity or debugging, suppress at the call site:

```rust
#[allow(ledger_context_read_in_loop)]
let seq = env.ledger().sequence();
```
