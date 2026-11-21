# `map_iter_then_lookup`

| Default Severity | Category |
| ---------------- | -------- |
| `warn`           | Compute  |

**Target Resource:** [Compute](../cost_rationale.md#1-cpu-instructions)

## What it does

Detects `for` loops iterating over `map.keys()` where the loop body immediately looks up each key on the same `soroban_sdk::Map` instance using `.get()` or `.get_unchecked()`.

## Why is this bad?

Writing `for k in map.keys() { map.get(&k) }` iterates over the key set and then performs an independent O(log n) host lookup for every key, paying quadratic host traversal cost O(n log n). Soroban contracts pay metered host CPU cost for each host call. Using `map.iter()` provides direct access to both key and value in a single pass in O(n) linear time.

## Example

```rust
// Bad: quadratic traversal
for k in map.keys() {
    let val = map.get(&k);
}

// Good: direct iteration over entries
for (k, val) in map.iter() {
    // use k and val directly
}
```

## Known limitations

- Only reports loops where the receiver of `.keys()` and `.get()` / `.get_unchecked()` refers to the same map.
