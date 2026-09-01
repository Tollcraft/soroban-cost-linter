# `instance_storage_write_in_loop`

## What it does

Flags `.instance().set(...)` calls that sit inside a loop body. Instance storage is a single ledger entry holding the contract's entire instance map. Writing to it does not update one field — it serialises and writes the whole entry. Doing that inside a loop rewrites the full instance state on every iteration, so a loop updating ten counters pays ten full instance-entry writes where one write after the loop would do.

## Why is this bad?

Soroban meters execution against a CPU and memory budget. Instance storage serialises and rewrites the entire instance map on every `set` call. When this happens inside a loop, each iteration pays the full serialisation and write cost for the complete map — not just the changed field. A loop that updates N fields pays N full instance-entry writes. Accumulating changes in local variables and writing once after the loop reduces this to one write.

## Example

```rust
// BAD: rewrites the full instance map on every iteration
for i in 0..10 {
    let existing: Option<u32> = env.storage().instance().get(&i);
    env.storage().instance().set(&i, &(existing.unwrap_or(0) + 1));
}

// GOOD: accumulate in a local, write once after the loop
let mut updates = std::vec::Vec::new();
for i in 0..10 {
    let existing: Option<u32> = env.storage().instance().get(&i);
    updates.push((i, existing.unwrap_or(0) + 1));
}
for (i, val) in updates {
    env.storage().instance().set(&i, &val);
}
```

## Interaction with `soroban_storage_in_loop`

Both `soroban_storage_in_loop` and `instance_storage_write_in_loop` will fire on the same expression when instance storage is written inside a loop. The former gives general "storage in a loop" advice; the latter gives the specific accumulate-then-write advice that applies because instance storage rewrites the full entry.

## Known false positives

See [Handling False Positives](../false_positives.md#instance_storage_write_in_loop).

## Suppression

```rust
#[allow(instance_storage_write_in_loop)]
fn batch_update(env: Env) {
    for i in 0..10 {
        env.storage().instance().set(&i, &1);
    }
}
```
