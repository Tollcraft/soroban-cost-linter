# `collection_len_in_loop_condition`

**Default Severity:** `warn`

## What it does

Detects calls to `.len()` on Soroban collection types (`soroban_sdk::Vec`, `soroban_sdk::Map`) in the condition of a `while` loop when the collection is not mutated inside the loop body or the condition.

## Why is this bad?

{% hint style="danger" %}
On Soroban host-side collections, `.len()` is a metered host call — it crosses the contract–host boundary and incurs CPU cost each time it is invoked. A `while` loop re-evaluates its condition on every iteration, so when the collection is not modified inside the loop the length is constant across all iterations and re-evaluating it wastes one host call per iteration for a value that never changes.
{% endhint %}

## Example

```rust
// ❌ Bad: vec.len() re-evaluated every iteration
let vec: Vec<u32> = Vec::new(&env);
let mut i = 0;
while i < vec.len() {
    process(vec.get_unchecked(i));
    i += 1;
}
```

## Suggested Fix

{% hint style="success" %}
Bind the collection length to a local variable before the loop so the host call happens exactly once.
{% endhint %}

```rust
let vec: Vec<u32> = Vec::new(&env);
let len = vec.len(); // single host call
let mut i = 0;
while i < len {
    process(vec.get_unchecked(i));
    i += 1;
}
```

## When is this NOT flagged?

- **The collection is mutated in the loop.** The lint is suppressed when the collection is mutated inside the loop body or the condition (e.g. via `.push_back()`, `.set()`, `.insert()`, `.remove()`, `.pop_back()`, `.clear()`, `.truncate()`, or `.swap_remove()`), or when the binding is reassigned. In that case the length genuinely changes between iterations, so re-evaluation is load-bearing.

  ```rust
  // ✅ OK: vec is mutated, so len() must be re-evaluated
  let mut vec: Vec<u32> = Vec::new(&env);
  let mut i = 0;
  while i < vec.len() {
      vec.push_back(i);
      i += 1;
  }
  ```

- **The bound is a `for` range.** `for i in 0..vec.len()` evaluates the range once before the loop starts, so there is nothing to hoist and the lint stays silent.

- **Native `std` collections.** Only `soroban_sdk` types are host calls; `std::vec::Vec::len` is a plain memory read and is out of scope.

## Known limitations

The mutation check is structural: it looks for mutating method calls and assignments on the same local binding that the `len()` receiver names. Indirect mutations (through shared references, helper functions, or complex receiver expressions such as field accesses) are not detected, and a receiver that is not a plain local binding is treated as unmutated. This bias is intentional — false positives are more harmful than false negatives for an opt-in cost lint.
