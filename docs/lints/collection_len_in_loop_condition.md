# `collection_len_in_loop_condition`

**Default Severity:** `warn`

## What it does

Detects calls to `.len()` on Soroban collection types (`Vec`, `Map`, `Set`) inside loop bodies when the collection is not mutated within the loop.

## Why is this bad?

{% hint style="danger %}
On Soroban host-side collections, `.len()` is a metered host call — it crosses the contract–host boundary and incurs CPU cost each time it is invoked. When the collection is not modified inside the loop, the length is constant across all iterations, so re-evaluating it wastes one host call per iteration for a value that never changes.
{% endhint %}

## Example

```rust
// ❌ Bad: vec.len() re-evaluated every iteration
let vec: Vec<u32> = Vec::new(env);
let mut i = 0;
while i < vec.len() {
    process(vec.get_unchecked(i));
    i += 1;
}

// ❌ Bad: same problem with a for-range bound
let vec: Vec<u32> = Vec::new(env);
for i in 0..vec.len() {
    process(vec.get_unchecked(i));
}
```

## Suggested Fix

{% hint style="success %}
Bind the collection length to a local variable before the loop so the host call happens exactly once.
{% endhint %}

```rust
let vec: Vec<u32> = Vec::new(env);
let len = vec.len(); // single host call
let mut i = 0;
while i < len {
    process(vec.get_unchecked(i));
    i += 1;
}
```

## When is this NOT flagged?

The lint is suppressed when the collection is mutated inside the loop body (e.g. via `.push()`, `.set()`, `.insert()`, `.remove()`, `.pop()`, `.clear()`, `.truncate()`, or `.swap_remove()`). In that case the length genuinely changes between iterations, so re-evaluation is load-bearing.

```rust
// ✅ OK: vec is mutated, so len() must be re-evaluated
let vec: Vec<u32> = Vec::new(env);
let mut i = 0;
while i < vec.len() {
    vec.push(&i);
    i += 1;
}
```

## Known limitations

The mutation check only detects direct method calls on the same local variable. Indirect mutations (through shared references, returned values, or complex receiver expressions) are not detected. This is intentional — false positives are more harmful than false negatives for an opt-in cost lint.
