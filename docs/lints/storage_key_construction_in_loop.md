# `storage_key_construction_in_loop`

**Default Severity:** `warn`

**Target Resource:** [CPU — host function dispatch and execution, Memory — host allocations](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects `Symbol::new(&env, ...)` calls inside a loop body where the constructed key
does **not** depend on the loop variable. Such keys are loop-invariant and can be
hoisted outside the loop.

## Why is this bad?

{% hint style="danger" %}
`Symbol::new` allocates through the host on every call — it crosses the Wasm–host
boundary, allocates host-side memory, and registers the symbol. When the key is
the same across every iteration (loop-invariant), every call after the first is
pure waste. Hoisting the call outside the loop turns N host allocations into 1.
{% endhint %}

## Example

```rust
// ❌ Bad: the same key is constructed on every iteration
for _ in 0..10 {
    let key = Symbol::new(&env, "counter");
    let val = env.storage().instance().get(&key);
}
```

```rust
// ✅ Good: hoist the loop-invariant key outside the loop
let key = Symbol::new(&env, "counter");
for _ in 0..10 {
    let val = env.storage().instance().get(&key);
}
```

## Suggested Fix

{% hint style="success" %}
Hoist the `Symbol::new` call outside the loop, bind it to a local variable, and
reference that variable inside the loop.
{% endhint %}

## What is not reported

- Calls where the key **depends on the loop variable** (e.g. `Symbol::new(&env,
  &format!("key_{}", i))`). These are genuine per-iteration work and hoisting
  would change behaviour.
- Calls outside of syntactic loop bodies.

## Known limitations

The loop-invariance analysis used by this lint is the same
[`depends_on_loop_state`] helper shared with [`unnecessary_host_function_call`].
It has the following known gaps, all of which bias towards **not** reporting
(false negatives) rather than false positives:

- Bindings and mutations inside a closure body nested in the loop are not seen.
- Mutation through raw pointers or interior mutability (`Cell`, `RefCell`) is
  not tracked.
- The analysis operates on syntactic loops only; iterator adapters
  (`.for_each()`, `.map()`) are not inspected.

Only `Symbol::new` is matched in this initial implementation. Future revisions
may extend matching to other Soroban key types (`Bytes`, `Vec`, contracttype
key structs).

## Cost impact

Every `Symbol::new` call crosses the Wasm–host boundary and allocates host-side
memory. In a loop of N iterations, hoisting saves N − 1 host allocations.

Measured with `Env::default()` in the [`cost_benchmarks`](https://github.com/Tollcraft/soroban-cost-linter/tree/main/cost_benchmarks) crate (`cargo test -- --nocapture`):

| Pattern | Iterations | CPU instructions (delta) | Memory bytes (delta) |
| --- | --- | --- | --- |
| `Symbol::new` in loop (bad) | 100 | *run `cargo test -- --nocapture` in `cost_benchmarks/`* | *run `cargo test -- --nocapture` in `cost_benchmarks/`* |
| Hoisted: call once, reuse (good) | 100 | *run `cargo test -- --nocapture` in `cost_benchmarks/`* | *run `cargo test -- --nocapture` in `cost_benchmarks/`* |

### How to reproduce

```bash
cd cost_benchmarks
cargo test bench_storage_key_construction_in_loop -- --nocapture
```

## Deliberately not covered

- Other Soroban key types (`Bytes`, `Vec`, contracttype structs) — these may be
  added in future revisions.
- Auto-fix suggestions — the fix is context-dependent (hoisting point varies by
  code structure).
