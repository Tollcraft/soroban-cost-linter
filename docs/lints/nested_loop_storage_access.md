# `nested_loop_storage_access`

**Default Severity:** `deny`

**Target Resource:** [Storage — ledger entry accesses and ledger I/O bytes](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects storage operations (reads or writes) that are executed inside **nested**
loop bodies — i.e., at a loop nesting depth of 2 or more. A storage access at
depth 1 is linear in one dimension; at depth 2 it becomes multiplicative
(O(n·m)).

## Why is this bad?

{% hint style="danger" %}
Storage operations are the **single most expensive resource** Soroban charges
for. Each write consumes a ledger entry write access, I/O bytes, serialization
cost, and (for new entries) space rent. Placing them inside a loop multiplies
every dimension by the iteration count. Placing them inside **nested** loops
makes the cost **multiplicative** — a contract that passes testing with small
inputs can exceed the ledger's resource limits on real data. See the
[Cost Rationale — Storage](../cost_rationale.md#3-storage-ledger-entry-accesses-and-ledger-io)
for details.
{% endhint %}

## Example

### Nested `for` loops

```rust
// ❌ Bad: storage write at O(n·m) — every iteration of both loops
for i in 0..n {
    for j in 0..m {
        env.storage().instance().set(&(i + j), &1);
    }
}
```

### `for` loop containing a `while` loop

```rust
// ❌ Bad: storage read at O(n·m)
for i in 0..n {
    let mut j = 0;
    while j < m {
        let _val = env.storage().persistent().get(&i);
        j += 1;
    }
}
```

## Cost impact

| Pattern | Iterations | Cost model |
| --- | --- | --- |
| Storage in single loop | n | O(n) |
| Storage in nested loops | n × m | O(n·m) |

A 10×10 nested loop with a storage write in the inner body issues 100 storage
operations. The same operation in a single loop with 100 iterations issues 100
storage operations, but the nested version is harder to reason about and more
likely to be a mistake — the author typically intended the inner operation to
apply to a single aggregated result.

## Suggested Fix

{% hint style="success" %}
Hoist the storage operation out of at least one loop, or accumulate mutations
in memory and write once after the loops complete.
{% endhint %}

```rust
// ✅ Fixed: accumulate in memory, single write after loops
let mut total = 0i128;
for i in 0..n {
    for j in 0..m {
        total += i + j;
    }
}
env.storage().instance().set(&COUNTER, &total);
```

## How loop nesting depth is computed

Depth is computed by walking up the HIR parent chain from the storage
operation:

- **`for`, `while`, `loop`** — each increments the depth counter.
- **Closures** (e.g., `.iter().for_each(|x| { ... })`) — **not** counted as
  additional nesting. A closure body inside a loop is still inside the same
  loop; the closure is the loop's callback, not a separate nesting level.
- **Function definitions** — stop the walk. A `fn` defined inside a loop is a
  separate function that may be called from anywhere, so its internal loops
  are independent.

### Examples of correct depth computation

| Pattern | Computed depth | Fires? |
| --- | --- | --- |
| `for i in 0..n { storage.set() }` | 1 | No |
| `for i in 0..n { for j in 0..m { storage.set() } }` | 2 | **Yes** |
| `for i in 0..n { items.iter().for_each(\|x\| { storage.set() }) }` | 1 | No |
| `for i in 0..n { for j in 0..m { items.iter().for_each(\|x\| { storage.set() }) } }` | 2 | **Yes** |
| `for i in 0..n { fn inner() { for j in 0..m { storage.set() } } inner(); }` | 1 | No |

## Relationship to `soroban_storage_in_loop`

| Lint | Default Severity | What it flags |
| --- | --- | --- |
| [`soroban_storage_in_loop`](soroban_storage_in_loop.md) | `warn` | **Every** storage operation inside any loop, regardless of nesting depth |
| `nested_loop_storage_access` | `deny` | Storage operations at **depth ≥ 2** — the multiplicative-cost case |

`soroban_storage_in_loop` fires on all storage-in-loop patterns, including
depth-1 cases where the cost is linear. `nested_loop_storage_access` is a
strict subset that fires only when the storage access is inside nested loops,
where the cost becomes multiplicative. The two lints are complementary:
`soroban_storage_in_loop` catches the general case;
`nested_loop_storage_access` highlights the shape most likely to exceed
resource limits on real data.

When a storage operation is at depth ≥ 2, **both** lints may fire. This is
deliberate — `nested_loop_storage_access` carries a stronger severity (`deny`)
and a more specific diagnostic to make the quadratic cost explicit.

## What is **not** reported

- A storage operation inside a single loop (depth 1) — handled by
  [`soroban_storage_in_loop`](soroban_storage_in_loop.md).
- A storage operation inside a closure passed to an iterator method inside a
  single loop — the closure is the loop body, not a separate nesting level.
- A storage operation inside a function definition that itself sits inside a
  loop — the inner function may be called from anywhere.
- A storage operation outside all loops.

## Known False Positives

This lint is intentionally conservative about what constitutes a nesting level.
The following patterns are correctly **not** flagged but may look surprising:

- A closure passed to `.iter().for_each()` inside a loop — the closure is the
  loop's iteration callback, not a nested loop.
- A storage operation inside a `match` arm within a loop — `match` is not a
  loop construct.

If a pattern is intentionally nested and the multiplicative cost is desired,
suppress with `#[allow(nested_loop_storage_access)]`.

## See also

- [`soroban_storage_in_loop`](soroban_storage_in_loop.md) — the broader lint that catches all storage-in-loop patterns
- [`loop_invariant_storage_access`](loop_invariant_storage_access.md) — loop-invariant storage operations that can be hoisted
- [Cost Rationale](../cost_rationale.md) — why storage operations dominate Soroban fees
