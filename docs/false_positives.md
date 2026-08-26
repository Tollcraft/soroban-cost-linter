# Handling False Positives

Static analysis tools occasionally flag code that is intentionally written the way it is. This guide explains how to recognize, suppress, evaluate, and track false positives in `soroban-cost-linter`.

## What is a False Positive?

A false positive is a lint warning that fires on code that does not actually contain the problem the lint is designed to catch, or where the flagged cost is intentional, unavoidable, or inherent to the contract's business logic.

For example, `soroban_storage_in_loop` warns when a storage operation appears inside a loop body. In most code this is an expensive anti-pattern, but if you are intentionally writing different keys on each iteration (e.g., writing a batch of entries), the warning is a false positive — the code is correct, and the cost is inherent to the operation.

---

## Real-World Corpus Baseline & Tracking

The project runs continuous regression and triage checks against real-world Soroban contracts in `tests/corpus/` via `cargo-cost-lint/tests/real_world_corpus.rs`. The findings are recorded in `tests/corpus/baseline.json`.

### Current Corpus Baseline Statistics

| Metric | Count | Percentage |
|---|---:|---:|
| **Total Findings** | 90 | 100.0% |
| **True Positives (TP)** | 17 | 18.9% |
| **False Positives (FP)** | 73 | 81.1% |

### Breakdown by Lint

| Lint | False Positives | True Positives | Default Level | Tracking / Decision |
|---|---:|---:|---|---|
| `loop_invariant_storage_access` | 23 | 0 | warn | Tracking precision enhancements for receiver-chain hoisting and loop-variant arguments |
| `soroban_storage_in_loop` | 16 | 0 | warn | Intentional batch-write patterns; key variance analysis under design |
| `storage_write_without_read` | 13 | 0 | warn | Blind overwrites and initialization flows across multi-tx invocations |
| `vec_where_slice_could_be_used` | 9 | 0 | warn | Public interface entrypoints requiring SDK collections vs internal helpers |
| `storage_key_construction_in_loop` | 4 | 0 | warn | Dynamic key construction in loop iterations |
| `bytes_append_in_loop` | 3 | 0 | warn | Intentional growing buffers; recommend preallocating where possible |
| `instance_storage_for_unbounded_data` | 3 | 0 | warn | Collections bounded by contract invariants; storage footprint limit |
| `soroban_inefficient_bytes_concat` | 1 | 0 | warn | String/bytes formatting in loop iterations |
| `contract_call_in_loop` | 1 | 0 | warn | Cross-contract batch dispatches |
| `symbol_new_for_short_literal` | 0 | 8 | warn | True positive: short literals should use `symbol_short!` |
| `unwrap_on_storage_get` | 0 | 4 | warn | True positive: direct unwrap on storage read |
| `redundant_env_clone` | 0 | 3 | warn | True positive: redundant clones on `Env` handles |
| `unnecessary_host_function_call` | 0 | 2 | warn | True positive: host functions callable outside loops |

### Target False-Positive Ratio & Policy

1. **Long-Term Target Ratio:**
   Our goal is to reduce the corpus false-positive rate below **20%** across real-world contracts as dataflow, alias, and AST/HIR precision analyses mature.
2. **Regression Guard (CI Gate):**
   The baseline test (`real_world_corpus`) acts as a regression gate in CI.
   - **FP Increases:** Any change that increases the number of false positives across the corpus will fail CI. Such changes must either be refined or accompanied by an explicit issue and maintainer approval.
   - **FP Reductions:** When a lint precision improvement reduces false positives, the contributor must re-bless the baseline with `BLESS=1 cargo test --test real_world_corpus --workspace` and commit the updated `tests/corpus/baseline.json`.

---

## Known False Positive Patterns by Lint

### `loop_invariant_storage_access`

Flags storage method calls (`env.storage()`, `.instance()`/`.persistent()`/`.temporary()`, and the terminal `get`/`has`/`set`) inside a loop whose operands are provably loop-invariant.

- **Chain Warnings:** A single logical access `env.storage().instance().get(&1)` emits three warnings (one each for `storage()`, `instance()`, `get()`) because each call in the chain is evaluated independently.
- **Loop-Variant Arguments with Constant Receivers:** When a call like `get(item)` varies with the loop variable `item`, the terminal `get` call is suppressed, but `env.storage().instance()` calls are still flagged if `env` is invariant. Hoist `let instance = env.storage().instance();` outside the loop to resolve.
- **Intentional Dynamic Storage:** When storage access within a loop is intentional, suppress with `#[allow(loop_invariant_storage_access)]`.

### `soroban_storage_in_loop`

Every storage read or write inside any loop body is flagged.

- **Batch writes with different keys** — iterating over a collection and writing each element under a different storage key.
- **Storage reads that depend on the loop variable** — reading a value for each item in a collection, where the key changes per iteration.
- **Counting or scanning patterns** — using a loop to count entries or scan through storage with `has()`.
- **Handling:** Suppress intentional batch operations using `#[allow(soroban_storage_in_loop)]`.

### `storage_write_without_read`

Fires on any `set` whose `(receiver, key)` snippet has no matching `get`/`has` anywhere in the same function.

- **Near-miss — initializer skip:** Functions named `init` or `set_admin` are intentionally skipped.
- **Cross-Function & Multi-Transaction Overwrites:** Storage written blindly as an update or status reset without reading first within the same function is flagged. If the overwrite is intentional, suppress with `#[allow(storage_write_without_read)]`.
- **Syntactic Snippet Mismatch:** If the key expression in `has(&key)` is written differently from `set(key)` (e.g. referencing with/without `&`), the syntactic matcher will not correlate them.

### `vec_where_slice_could_be_used`

Fires when a function parameter takes `soroban_sdk::Vec<T>` by value rather than a native Rust slice `&[T]`.

- **Public Contract Entrypoints:** Contract trait methods and exported functions must accept `soroban_sdk::Vec` to be callable across Soroban boundaries. For public interface entrypoints, suppress with `#[allow(vec_where_slice_could_be_used)]`.
- **Internal Helper Functions:** Internal helpers should take `&[T]` or `&Vec<T>` to avoid host object creation and cloning overhead.

### `storage_key_construction_in_loop`

Flags constructing storage keys (such as `Symbol::new`, enum data keys, or tuple keys) inside loop bodies where the key is invariant.

- **Hoisting:** Where the key is constant across iterations, hoist its construction outside the loop.
- **Iteration-Dependent Keys:** If key construction depends on the loop index or element, suppress with `#[allow(storage_key_construction_in_loop)]`.

### `bytes_append_in_loop`

Flags calling `.append()` or `.push_back()` on `Bytes` or `Vec` inside loops.

- **Host Reallocation Cost:** In Soroban, growing SDK containers allocates new host objects per iteration.
- **Remedy:** Preallocate collections where length is known or accumulate natively before creating host objects. If incremental host appending is required, suppress with `#[allow(bytes_append_in_loop)]`.

### `instance_storage_for_unbounded_data`

Flags writing collections (e.g. `Vec`, `Map`) to `instance` storage without an evident size bound.

- **Footprint Risk:** Instance storage is limited to 64KB per contract and shares a single TTL with the contract executable.
- **Handling:** Prefer `persistent` or `temporary` storage for dynamically growing user data. If the collection has an enforced invariant bound (e.g., maximum 10 elements), suppress with `#[allow(instance_storage_for_unbounded_data)]`.

### `soroban_inefficient_bytes_concat` / `inefficient_bytes_concat`

Flags incremental concatenation of byte sequences inside loops.

- **Remedy:** Pre-calculate required buffer size and construct host `Bytes` once.

### `contract_call_in_loop`

Flags cross-contract invocations (`Client::new(&env, &addr).method(...)`) inside loops.

- **Overhead:** Each cross-contract call invokes host context switching and separate auth/cost accounting.
- **Handling:** Batch cross-contract calls where possible; suppress with `#[allow(contract_call_in_loop)]` when per-item cross-contract dispatch is required.

### `unnecessary_host_function_call`

Flags host function calls inside loops whose arguments do not depend on loop state.

- **Bindings in Closures:** Mutations inside a closure nested within the loop are not tracked.
- **Interior Mutability:** Mutability through `RefCell` or raw pointers is not tracked.
- **Intentional Calls:** Calls like `env.prng().u64_in_range()` with constant bounds are flagged; suppress with `#[allow(unnecessary_host_function_call)]`.

### `redundant_env_clone`

Fires on `.clone()` calls on `Env`. `Env` is a lightweight copyable handle.

- **Consumed Env:** Where `Env` is consumed by value before a clone site, or in generic contexts that do not guarantee copy semantics.

### `symbol_new_for_short_literal`

Fires when `Symbol::new(&env, "short")` is used with a literal string <= 9 characters.

- **Remedy:** Replace with `symbol_short!("short")` for zero host overhead at runtime.

### `unbounded_recursion`

Flags recursive function cycles whose depth is caller-supplied.

- **Structurally Bounded Helpers:** When custom collection methods advance through a sub-slice not recognized by the built-in tail set, suppress with `#[allow(unbounded_recursion)]`.

### `soroban_redundant_storage_read`

Fires when two reads of the same key appear with no intervening write in the same block.

- **Block Scoping:** Reads across distinct conditional branches or nested closures are not treated as duplicate reads.

### `persistent_read_without_ttl_extension`

Fires on persistent storage reads when the containing function does not invoke `extend_ttl`.

- **Function-Wide Check:** A single `extend_ttl` call on persistent storage in the function satisfies the lint.

### `unwrap_on_storage_get`

Fires on `.unwrap()` / `.expect()` called directly on a storage read.

- **Immediate Overwrite:** If a key was just set in the same transaction, suppress with `#[allow(unwrap_on_storage_get)]` or use pattern matching `if let Some(...)`.

---

## Suppression Methods

You have three layers of suppression, each suited to a different scope.

### 1. Per-site: `#[allow(...)]` Attribute

Suppress the lint for a single function, expression, or block:

```rust
#[allow(soroban_storage_in_loop)]
fn batch_write(env: Env, items: Vec<u32>) {
    for item in items {
        env.storage().instance().set(&item, &1);
    }
}
```

This is the most targeted suppression. Use it when the flagged code is intentional and the lint gives no other way to express that intent.

You can also use `#[expect(...)]` (nightly Rust) to verify that the lint fires — the compiler will warn if the lint *stops* firing when a newer linter release resolves the pattern:

```rust
#[expect(soroban_storage_in_loop)]
fn batch_write(env: Env, items: Vec<u32>) {
    for item in items {
        env.storage().instance().set(&item, &1);
    }
}
```

### 2. Per-file: `.lintignore`

Create a `.lintignore` file in your workspace root (next to `Cargo.toml`). The linter respects standard glob patterns:

```gitignore
# Ignore all lint warnings in generated files
src/generated/*.rs

# Ignore deliberately expensive legacy modules
src/legacy_batch.rs
```

### 3. Per-workspace: `budget.toml`

Set a lint's severity to `"allow"` in `budget.toml` to disable it project-wide:

```toml
[lints]
soroban_storage_in_loop = "allow"
```

Use workspace-wide suppression sparingly — prefer targeted `#[allow(...)]` attributes or `.lintignore` rules.

---

## How to Evaluate a False Positive

Before suppressing a warning, follow this decision checklist:

1. **Is the cost real and unavoidable?** — Does eliminating the warning require changing the contract algorithm, or is it an inherent property of batch operations? If unavoidable, suppress. If hoistable, refactor.
2. **Can the code be restructured?** — Hoisting invariant expressions (such as storage handles or key constructions) outside loops eliminates overhead without changing behavior.
3. **Is the pattern covered by Clippy?** — Check the [Scope Boundary](scope_boundary.md) guide for general Rust vs Soroban-specific patterns.

---

## Reporting False Positives Upstream

If a lint produces a false positive that should be handled automatically by static analysis:

1. Search existing issues on GitHub to avoid duplicates.
2. Provide a minimal reproducible example (a standalone Soroban contract function).
3. State the lint name, the expected vs actual diagnostic, and why the code is optimal.
4. Mention the `soroban-cost-linter` version and nightly toolchain pin.
