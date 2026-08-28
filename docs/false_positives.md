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
| `string_concat_in_loop` | 0 | 0 | warn | New lint; not yet present in the corpus baseline — pending first corpus run |
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

### `string_concat_in_loop`

Flags `append` on (or `String + String` addition of) a `soroban_sdk::String` inside loops.

- **Host Reallocation Cost:** Each concatenation allocates a fresh host buffer and copies the entire accumulated string, so building a string from `n` pieces inside a loop is O(n²) in the number of characters produced.
- **Small Fixed-Bound Loops (Known False Positive):** The lint does **not** prove loop bounds — it fires on any syntactic loop, mirroring `bytes_append_in_loop`. A loop with a small, fixed iteration count (e.g. 2–3) is the documented false positive; suppress the specific call site with `#[allow(string_concat_in_loop)]` or accumulate the few pieces in a native `Vec` and construct the `String` once.
- **Remedy:** Accumulate the pieces in a native collection (e.g. `Vec<String>` or `Vec<Bytes>`) inside the loop and construct the `String` a single time afterwards; pre-size where practical.

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
- **Boundary:** the fixture pins the length boundary — a literal of exactly 9 characters fires (the maximum accepted by `symbol_short!`), a 10-character literal does not.
- **Near-miss — invalid characters:** literals containing characters outside `[a-zA-Z0-9_]` (e.g. `-`, spaces) are not flagged, because `symbol_short!` would not accept them either.
- **Near-miss — non-literal argument:** `Symbol::new(&env, s)` where `s` is a variable is not flagged; the lint only matches string-literal arguments.
- **Near-miss — empty literal:** `Symbol::new(&env, "")` is not flagged.

### `map_insert_in_loop`

Flags `Map::insert` calls on `soroban_sdk::Map` inside any loop body.

- **Host reallocation cost:** each `insert` mutates a host-side map object, and per-iteration inserts are increasingly expensive as the map grows.
- **Handling:** accumulate mutations in a native `Vec<(K, V)>` inside the loop and build the `Map` once after the loop, or suppress with `#[allow(map_insert_in_loop)]` when per-iteration insertion is intentional.

### `storage_key_construction_in_loop`

Flags `Symbol::new(&env, ...)` calls inside a loop body whose key does not depend on the loop variable.

- **Genuine finding — loop-invariant key:** `let key = Symbol::new(&env, "my_key");` inside a loop reconstructs the same host object every iteration. Hoist the construction outside the loop.
- **Near-miss — loop-variant key:** `Symbol::new(&env, &format!("key_{}", i))` inside a loop reads the loop variable `i`, so the lint correctly does not fire — the key genuinely varies per iteration.
- **Handling:** hoist invariant key construction outside the loop. Suppress with `#[allow(storage_key_construction_in_loop)]` when per-iteration key construction is intentional.

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

### `std_collection_in_contract`

Fires on `std::collections::HashMap`, `std::collections::BTreeMap`, and `std::vec::Vec` usage inside a `#[contractimpl]` block.

- **Known false positive — performance-critical inner loop with small, fixed-size data:** if the contract processes a tiny, fixed, contract-controlled collection (e.g. a 2–3 element lookup table that never grows), the overhead of host-boundary conversion may exceed the cost of wasm-linear-memory allocation. Suppress with `#[allow(std_collection_in_contract)]` for such cases.
- **Near-miss 1 — helper function called from `#[contractimpl]`:** the lint only fires inside the `#[contractimpl]` block itself, not in helper functions called from it. A helper that uses `HashMap` for internal bookkeeping is not flagged, which is intentional — the boundary is narrow and correct.
- **Near-miss 2 — non-collection std types:** `String`, `Box`, `Rc`, `Arc`, and other std types are not flagged. Only the three collection types listed above are in scope.
- **Test code exclusion:** std collections in `#[test]` functions and `#[cfg(test)]` modules are never flagged, because they are idiomatic and correct in tests.

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
\n

The lint's mutation analysis (used by `unnecessary_host_function_call`) is the area most likely to improve; regression tests from real-world false positives are particularly valuable.

## Verifying Suppression in Tests

When you suppress a lint, verify that the suppression works correctly:

1. **With `#[allow(...)]`** — compile with the attribute. The lint should not fire. Remove the attribute and confirm the lint does fire (to prove the code would have been flagged).
2. **With `.lintignore`** — run `cargo cost-lint` with and without the `.lintignore` entry to confirm the finding appears or disappears.
3. **With `budget.toml`** — set the level to `"allow"` and confirm `cargo cost-lint` exits with code 0 even when the pattern is present.

## Summary

| Scope | Method | Best for |
|-------|--------|----------|
| Per-site | `#[allow(lint_name)]` | Intentional patterns at specific call sites |
| Per-file | `.lintignore` | Generated code, vendored deps, entire files |
| Per-workspace | `budget.toml` `"allow"` | Project-wide decisions (use sparingly) |

### `loop_invariant_storage_access`

This lint flags storage method calls (`env.storage()`, `.instance()`/`.persistent()`/`.temporary()`, and the terminal `get`/`has`/`set`) that sit inside a loop and whose operands are provably loop-invariant. Notes from writing the fixture:

- A single logical `env.storage().instance().get(&1)` inside a loop emits **three** warnings — one per call in the chain (`storage`, `instance`, then `get`) — because each call is matched independently and each is loop-invariant. This is expected, not a defect.
- **Genuine near-miss (must not fire):** when the *receiver* is the loop variable — `for s in stores.iter() { s.get(&1); }` where `s: &Instance` — the access depends on loop state and is correctly skipped. This is the real "varies per iteration" case.
- **Subtler near-miss:** a `get(item)` whose *argument* `item` is the loop variable. The `get` call is correctly skipped (its argument depends on loop state), but the `env.storage()` and `.instance()` receiver calls are *still* flagged, because their receiver `env` is constant and therefore loop-invariant. So "the key varies per iteration" does not fully silence the lint — only the terminal call is suppressed. This is documented so the surviving receiver warnings are not mistaken for a bug.
- The lint keys off structural loop-invariance, not value ranges; a literal key (`&1`) is treated the same as `let k = &1; get(k)`.

### `soroban_redundant_storage_read`

Fires when two reads of the same key (by source-text snippet) appear with no intervening write, at the top level of a block.

- **Near-miss 1 — write between reads:** `get(&1); set(&1, &2); get(&1)`. The write resets the tracked key, so the second read is **not** flagged. Correct.
- **Near-miss 2 — different keys:** `get(&1); get(&2)`. Different source-text keys, so no redundancy is reported. Correct.
- Key equality is **textual** (the `snippet_opt` of the key argument), not semantic. `get(&k)` and `get(&k)` match; `get(&1)` and `get(1)` (without the `&`) would not, because the snippets differ. The check is syntactic — keep this in mind when reading the fixture.
- The lint only compares reads that are top-level statements/expressions within the **same** block. A read inside a nested `if`/`match`/closure is in a different block and is not compared against an outer-block read of the same key; that is why the fixture keeps both reads at the same block level.
- **No known false positives:** in every case the warning corresponds to a real duplicate read.

### `storage_write_without_read`

Fires on any `set` whose `(receiver, key)` snippet has no matching `get`/`has` anywhere in the same function.

- **Near-miss — initializer skip:** a function whose name contains `init` or `set_admin` is intentionally not analyzed, so a legitimate initializing `set` with no prior read stays silent. The fixture exercises this with `fn initialize(...)` and `fn set_admin(...)`.
- **Near-miss 2 — read precedes write:** `get(&1); set(&1, &2)`. The prior read of the same key suppresses the warning. Correct.
- Matching is by source-snippet text, so a read written with a syntactically different but semantically equal key (e.g. `has(&key)` paired with `set(key)` without the `&`) will not link and the write will still fire. The fixture uses identical snippets to exercise the matching path.
- Analysis is **per-function**: reads in a different function do not count toward a write's read set.
- **No known false positives** beyond the intentional initializer skip: any write with a truly absent read is reported, which is the lint's purpose.

### `persistent_read_without_ttl_extension`

Fires on every `get`/`has` on `persistent` storage when the function contains no `extend_ttl` call.

- **Near-miss 1 — TTL extended:** a single `extend_ttl(...)` call anywhere in the function suppresses **all** persistent-read warnings for that function. The check is all-or-nothing per function, not per key — so a function that extends TTL on one key but reads others without extending still produces no warning. Documented because it is easy to misread the fixture as per-key.
- **Near-miss 2 — non-persistent storage:** `instance.get(...)` / `temporary.get(...)` are out of scope and never flagged.
- The lint collects reads via a visitor over the whole function body, so a read in a nested block still counts.
- **No known false positives:** a persistent read with no `extend_ttl` in the function is always reported.

### `crypto_hash_of_constant`

Fires when `env.crypto().sha256(...)` / `env.crypto().keccak256(...)` is called with a literal or `const` item as its argument.

- **Genuine findings:** any `sha256(b"domain tag")` / `keccak256(&PREFIX_CONST)` call re-runs an expensive metered host hash to recompute a digest that is fixed at compile time. These are real wins — precompute the digest offline and embed it as a `const`.
- **Out of scope (deliberately silent):** a constant that is first wrapped in a constructor, e.g. `env.crypto().sha256(&Bytes::from_slice(&env, b"prefix"))`, is *not* flagged. The argument is a method call, not a literal/`const` item, so the lint conservatively stays quiet rather than risk a false positive. If your contract does this with a truly fixed prefix, precomputing the digest is still the right fix.
- **Known false positive — uniform code path:** some contracts intentionally hash a constant inside a helper that also hashes runtime data, so that *every* caller (constant and runtime alike) goes through one code path. For example:

  ```rust
  fn tagged_hash(env: &Env, prefix: &[u8], data: &[u8]) -> BytesN<32> {
      let combined = /* prefix || data */;
      env.crypto().sha256(&combined) // flagged only when `prefix` is the constant call-site's arg
  }
  // Caller that passes a compile-time constant prefix still pays for a runtime hash here.
  ```

  When the constant branch is deliberate — keeping a single, uniform hashing path for clarity or to share post-hash logic — suppress with `#[allow(crypto_hash_of_constant)]` at the call site, or split the constant case into its own precomputed `const` digest. This is the one pattern where the warning is correct in isolation but unwanted in context.



