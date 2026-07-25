# `event_in_loop`

**Default Severity:** `warn`

## What it does

Detects contract event emissions (`env.events().publish(...)`,
`env.events().publish_event(...)`, and the `env.events()` accessor itself)
that occur inside the body of a `for`, `while`, or `loop` construct.

## Why is this bad?

{% hint style="danger" %}
Each `env.events().publish` call crosses the guest/host boundary and
serializes the topics + data into a contract event in the transaction
footprint. Repeat this per-iteration and you pay that cost on every
iteration — directly driving up CPU and resource fees, and bloating the
event stream that off-chain consumers must index.
{% endhint %}

This is the same input-independent anti-pattern that
`soroban_storage_in_loop` catches for storage, applied to the events
surface of the contract.

## Example

```rust
// ❌ Bad: an event emit on every iteration
for i in 0..10 {
    env.events().publish((Symbol::new(&env, "tick"), i), &1u32);
}
```

## Suggested Fix

{% hint style="success" %}
Buffer the work in memory inside the loop, then publish a single event
(e.g. one summary event with a `soroban_sdk::Vec` payload) **after** the
loop. You keep the same observability story with a single host crossing.
{% endhint %}

```rust
// ✅ Fixed: one publish after the loop
let mut ticks: soroban_sdk::Vec<u32> = soroban_sdk::Vec::new(&env);
for i in 0..10 {
    ticks.push_back(i);
}
env.events().publish((Symbol::new(&env, "ticks"),), &ticks);
```
