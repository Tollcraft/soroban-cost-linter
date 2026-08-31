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
| **Total Findings** | 102 | 100.0% |
| **True Positives (TP)** | 26 | 25.5% |
| **False Positives (FP)** | 76 | 74.5% |

### New Cross-Contract Corpus Contracts Triage

| Lint | False Positives | True Positives | Default Level | Tracking / Decision |
|---|---:|---:|---|---|
| `loop_invariant_storage_access` | 23 | 0 | warn | Tracking precision enhancements for receiver-chain hoisting and loop-variant arguments |
| `soroban_storage_in_loop` | 16 | 0 | warn | Intentional batch-write patterns; key variance analysis under design |
| `storage_write_without_read` | 14 | 0 | warn | Blind overwrites and initialization flows across multi-tx invocations |
| `vec_where_slice_could_be_used` | 11 | 0 | warn | Public interface entrypoints requiring SDK collections vs internal helpers |
| `storage_key_construction_in_loop` | 4 | 0 | warn | Dynamic key construction in loop iterations |
| `bytes_append_in_loop` | 4 | 0 | warn | Intentional growing buffers; recommend preallocating where possible |
| `string_concat_in_loop` | 0 | 0 | warn | New lint; not yet present in the corpus baseline — pending first corpus run |
| `instance_storage_for_unbounded_data` | 3 | 0 | warn | Collections bounded by contract invariants; storage footprint limit |
| `soroban_inefficient_bytes_concat` | 0 | 2 | warn | True positive: inefficient Bytes concatenation inside a loop |
| `contract_call_in_loop` | 1 | 0 | warn | Cross-contract batch dispatches |
| `symbol_new_for_short_literal` | 0 | 10 | warn | True positive: short literals should use `symbol_short!` |
| `unwrap_on_storage_get` | 0 | 4 | warn | True positive: direct unwrap on storage read |
| `redundant_env_clone` | 0 | 3 | warn | True positive: redundant clones on `Env` handles |
| `unnecessary_host_function_call` | 0 | 2 | warn | True positive: host functions callable outside loops |
| `u128_where_u64_suffices` | 0 | 0 | warn | True positive: provably narrow 128-bit arithmetic operations on wasm32 |
| `storage_read_never_written` | 0 | 0 | warn | New lint; not yet present in the corpus baseline — pending first corpus run |
| `float_arithmetic_in_contract` | 0 | 0 | warn | New lint; not yet present in the corpus baseline — pending first corpus run |
| `duplicate_storage_key_construction` | 0 | 0 | warn | New lint; not yet present in the corpus baseline — pending first corpus run |

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

### `nested_loop_storage_access`

Fires on storage operations at loop nesting depth ≥ 2 — i.e., a storage access inside two or more nested loops.

- **Nested loop with intentional per-iteration writes** — writing to different keys in both loops where the multiplicative cost is inherent to the algorithm.
- **Closures inside nested loops** — a closure body inside a nested loop that performs a storage access; the closure is the inner loop's body, not a separate nesting level.
- **Handling:** If the nested storage access is intentional and the multiplicative cost is acceptable, suppress with `#[allow(nested_loop_storage_access)]`.

### `storage_write_without_read`

Fires on any `set` whose `(receiver, key)` snippet has no matching `get`/`has` anywhere in the same function.

- **Near-miss — initializer skip:** Functions named `init` or `set_admin` are intentionally skipped.
- **Cross-Function & Multi-Transaction Overwrites:** Storage written blindly as an update or status reset without reading first within the same function is flagged. If the overwrite is intentional, suppress with `#[allow(storage_write_without_read)]`.
- **Syntactic Snippet Mismatch:** If the key expression in `has(&key)` is written differently from `set(key)` (e.g. referencing with/without `&`), the syntactic matcher will not correlate them.

### `storage_read_never_written`

Fires at a storage **read** site when the key is never written by a *statically-known* `set`/`has` anywhere else in the same crate. It is inherently heuristic — it accumulates reads and writes across the whole crate and reports only at the end — so a clean false-positive story matters more than for single-body lints.

- **Cross-Contract State Sharing (the dominant false positive):** A contract routinely reads a key that another contract in the system writes. Factories, registries, and token/ledger adapters all rely on one contract reading state initialised by a different deployment. The lint cannot see across crate boundaries, so this read looks "never written" even though it is correct by design. This is the single most important false positive class for this lint and the reason it defaults to `warn` (not `deny`): the message explicitly says the write may live in another contract and that this is a warning, not proof of a bug.
- **Dynamically Constructed Keys:** When a key is built from a parameter, a computed value, or other runtime input, its value is unknown at analysis time. Such reads do **not** fire (we can't prove the key is unwritten) **and** do not suppress findings about unrelated static keys — a dynamic read and a static read-never-written can coexist, and only the static one is reported.
- **Distinct Key Spaces:** `instance`, `persistent`, and `temporary` storage are separate namespaces. A `persistent` write does not satisfy an `instance` read of the same literal key, so the read is still flagged. When the write legitimately lives in a different key space, this is a false positive to suppress with `#[allow(storage_read_never_written)]`.
- **Key-Name Typos:** The intended use case — a typo that turned one logical entry into two — is also the hardest to confirm automatically, which is why the diagnostic is phrased as a warning rather than an accusation.
- **Handling:** If the read is expected to be populated by another contract or by dynamic state, suppress with `#[allow(storage_read_never_written)]` at the read site or crate level.

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

### `option_wrapping_in_storage`

Fires when the value argument to a storage `.set()` call has type `Option<T>`.

- **Intentional Tri-State Entry:** A contract deliberately stores a tri-state value: missing (key absent), `Some(T)` (present with value), and `None` (present but empty). This is the documented deliberate false positive. For example, a registry that tracks whether a participant has ever registered (missing), is currently active (`Some(active_config)`), or has been explicitly deactivated (`None`).
- **Silencing:** Suppress at the call site with `#[allow(option_wrapping_in_storage)]`.

### `u128_where_u64_suffices`

Flags 128-bit arithmetic on values provably within 64 bits.

- **Token Balances & External Inputs:** Arithmetic derived directly from token balances, cross-contract calls, or caller-supplied `i128` parameters does not fire.
- **Handling:** If a 128-bit type is genuinely required by business logic across the entire expression, suppress with `#[allow(u128_where_u64_suffices)]`.

### `float_arithmetic_in_contract`

Flags arithmetic on `f32`/`f64` inside contract code.

- **Non-contract helper code:** Functions outside of `#[contractimpl]` blocks (test utilities, benchmarks) may legitimately use floats. The lint scope boundary is the `#[contractimpl]` attribute.
- **Explicit bit operations:** `f64::from_bits()` / `f64::to_bits()` for bit-level inspection do not perform arithmetic and are not flagged.
- **Handling:** If floating-point arithmetic is intentional and deterministic for your use case, suppress with `#[allow(float_arithmetic_in_contract)]`.

### `duplicate_storage_key_construction`

Fires when the same key expression is constructed in two or more distinct function bodies.

- **Cross-contract key sharing:** A factory contract may construct the same key pattern as a child contract, but the key spaces are separate. Suppress with `#[allow(duplicate_storage_key_construction)]`.
- **Intentional key derivation:** Some contracts intentionally build keys from different prefixes for different access patterns. Suppress at the call site.
- **Handling:** Hoist the key to a `const` or key enum. If the duplication is intentional, suppress with `#[allow(duplicate_storage_key_construction)]`.
### `ledger_context_read_in_loop`

Flags reading a ledger context value (`sequence`, `timestamp`, `network_id`) inside a loop.

- **Debugging/Logging in Loop:** A contract reads the ledger timestamp on each iteration for diagnostic logging or conditional branching based on ledger time. This is rare but intentional — the host call cost is accepted for observability. Suppress with `#[allow(ledger_context_read_in_loop)]`.
- **Relationship with `host_in_loop`:** A ledger context read inside a loop may also trigger `host_in_loop`. The `ledger_context_read_in_loop` lint provides a more specific explanation (the value is invariant during the invocation).
### `redundant_require_auth`

Fires when `Address::require_auth` or `Address::require_auth_for_args` is called more than once on the same address within a single function body, with no cross-contract call in between.

**Security caveat:** This lint advises about authorization. A false positive here is worse than a false positive in any other lint, because acting on it would remove a security check. The lint is intentionally conservative:

- **Address identity is compared by source-text snippet.** Two distinct variables holding the same address value are *not* flagged. This errs on the side of *not* flagging.
- **Cross-contract calls reset tracking.** Authorization context can change across `env.invoke_contract` / `env.try_invoke_contract` boundaries.
- **Cross-function analysis is out of scope.** If `require_auth` is called in two separate functions, the lint does not track across the function boundary.
