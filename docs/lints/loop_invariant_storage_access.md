# `loop_invariant_storage_access`

**Default Severity:** `warn`

**Target Resource:** [Storage — ledger entry accesses and ledger I/O bytes](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects storage operations (`get`, `set`, `has`) inside a loop whose key and value
operands are **provably loop-invariant** — i.e., they do not depend on the loop
induction variable or on any state mutated inside the loop body. When the operands
never change between iterations, the entire storage operation can be hoisted
outside the loop without altering behaviour.

## Why is this bad?

{% hint style="danger" %}
Storage operations are the **single most expensive resource** Soroban charges for.
Each storage write consumes a ledger entry write access, I/O bytes, serialization
cost, and (for new entries) space rent. Repeating a storage operation whose
operands never change multiplies every dimension by the iteration count for no
reason. See the [Cost Rationale — Storage](../cost_rationale.md#3-storage-ledger-entry-accesses-and-ledger-io) for details.
{% endhint %}

## Example

```rust
// ❌ Bad: same key and value written on every iteration
for _i in 0..10 {
    env.storage().instance().set(&"counter", &42);
}

// ❌ Bad: same key read on every iteration
for _i in 0..10 {
    let val = env.storage().persistent().get(&"config_key");
}
```

## Suggested Fix

{% hint style="success" %}
Hoist the storage operation out of the loop — perform it once before (or after)
the loop and reuse the result inside.
{% endhint %}

```rust
// ✅ Fixed: single storage write outside the loop
env.storage().instance().set(&"counter", &42);
for _i in 0..10 {
    // use the written value...
}

// ✅ Fixed: single storage read before the loop
let val = env.storage().persistent().get(&"config_key");
for _i in 0..10 {
    // reuse `val`...
}
```

## Relationship to `soroban_storage_in_loop`

| Lint | Default Severity | What it flags |
| ---- | ---------------- | ------------- |
| [`soroban_storage_in_loop`](soroban_storage_in_loop.md) | `warn` | **Every** storage operation inside a loop, regardless of whether its operands vary |
| `loop_invariant_storage_access` | `warn` | The **higher-confidence subset**: only storage operations whose operands are provably loop-invariant |

`soroban_storage_in_loop` is deliberately blunt — it catches the full class of
storage-in-loop anti-patterns but produces unavoidable false positives when the
key legitimately varies per iteration. `loop_invariant_storage_access` is the
precision complement: it only fires when the operands are proven invariant,
meaning the suggestion to hoist is always safe.

When both lints fire on the same call site, `loop_invariant_storage_access`
provides the more actionable suggestion ("hoist this out of the loop") while
`soroban_storage_in_loop` suggests the general mitigation ("accumulate in
memory first").

## What is **not** reported

A storage call is left alone when its key or value references:

- The loop induction variable (e.g., `for i in 0..n { ... .get(&i) }`)
- Any binding introduced inside the loop body
- Any variable mutated by the loop

Hoisting such a call would change behaviour, so this lint does not flag it.

When the mutation analysis cannot reach a verdict for a loop, every call in that
loop is left alone rather than reported on incomplete information.

## Known gaps

- Bindings and mutations inside a closure body nested in the loop are not seen
- Mutation through a raw pointer or through interior mutability (`Cell`,
  `RefCell`) is not tracked

In these cases a truly invariant call may be missed rather than incorrectly
reported.

## See also

- [`soroban_storage_in_loop`](soroban_storage_in_loop.md) — the blunt complement that flags all storage in loops
- [`unnecessary_host_function_call`](unnecessary_host_function_call.md) — the same loop-invariance analysis applied to host function calls
- [Cost Rationale](../cost_rationale.md) — why storage operations dominate Soroban fees
