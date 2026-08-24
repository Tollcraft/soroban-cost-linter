# Lint Categories

> Every lint in `soroban-cost-linter` is assigned one of five `LintCategory` values. This page explains what each category means, which metered Soroban resource it maps to, and how to choose one when adding a new lint.

The categories are defined by the `LintCategory` enum in `soroban_cost_lints/src/lib.rs`, and every shipped lint carries its category in the [`LINT_METADATA`](https://github.com/Tollcraft/soroban-cost-linter/blob/main/soroban_cost_lints/src/lib.rs) registry — the single source of truth. The `cargo-cost-lint` CLI reads that registry to enumerate lints (`--list`), to group warnings in the `--report` output, and to label `budget.toml` rows under their category. **If a documentation page and `LINT_METADATA` disagree, trust `LINT_METADATA`.**

## The five categories

| Category | What belongs in it | Soroban resource dimension | Shipped lints |
| --- | --- | --- | --- |
| `StorageOperations` | Direct ledger reads/writes via the `Storage`, `Instance`, `Persistent`, or `Temporary` accessors. | **Storage** — ledger entry accesses + ledger I/O bytes, the most expensive resource | [`soroban_storage_in_loop`](lints/soroban_storage_in_loop.md), [`loop_invariant_storage_access`](lints/loop_invariant_storage_access.md), [`soroban_redundant_storage_read`](lints/soroban_redundant_storage_read.md), [`storage_write_without_read`](lints/storage_write_without_read.md), [`instance_storage_for_unbounded_data`](lints/instance_storage_for_unbounded_data.md) |
| `Compute` | Host functions that cross the Wasm guest/host boundary and burn CPU budget with each call (`ledger`, `crypto`, `events`, ...). | **CPU** — Wasm instruction execution + host function dispatch | [`unnecessary_host_function_call`](lints/unnecessary_host_function_call.md), [`host_in_loop`](lints/host_in_loop.md), [`contract_call_in_loop`](lints/contract_call_in_loop.md), [`unbounded_input_loop`](lints/unbounded_input_loop.md), [`signature_verification_in_loop`](lints/signature_verification_in_loop.md), [`linear_scan_in_loop`](lints/linear_scan_in_loop.md), [`require_auth_in_loop`](lints/require_auth_in_loop.md), [`formatted_panic_payload`](lints/formatted_panic_payload.md) |
| `Memory` | Guest- or host-side allocations that grow with input size, including repeated `soroban_sdk::Bytes` / `Vec` / `Map` mutations. | **RAM** — guest linear memory, hard-capped but not charged in the resource fee | [`redundant_env_clone`](lints/redundant_env_clone.md), [`soroban_inefficient_bytes_concat`](lints/soroban_inefficient_bytes_concat.md), [`inefficient_bytes_concat`](lints/inefficient_bytes_concat.md), [`unnecessary_string_to_bytes`](lints/unnecessary_string_to_bytes.md), [`bytes_append_in_loop`](lints/bytes_append_in_loop.md), [`storage_key_construction_in_loop`](lints/storage_key_construction_in_loop.md), [`map_insert_in_loop`](lints/map_insert_in_loop.md), [`vec_where_slice_could_be_used`](lints/vec_where_slice_could_be_used.md) |
| `EntryLifecycle` | Lifecycle of contract entries: authorisation, deployment, removal. | **Storage** — ledger space rent / TTL paid when entries are created, extended, or removed | [`extend_ttl_in_loop`](lints/extend_ttl_in_loop.md), [`persistent_read_without_ttl_extension`](lints/persistent_read_without_ttl_extension.md) |
| `SymbolOperations` | Construction and reuse of `soroban_sdk::Symbol` values. | **CPU** — runtime symbol construction crosses the Wasm–host boundary | [`symbol_new_for_short_literal`](lints/symbol_new_for_short_literal.md) |

The "what belongs in it" descriptions above are taken verbatim from the `LintCategory` rustdoc, and the shipped-lint examples are the current `LINT_METADATA` assignments. The resource dimensions come from the [Cost Rationale](cost_rationale.md) page, which explains Soroban's full metering model — budgets, fees, and what dominates a contract's resource usage.

## How the categories map to the resource model

Soroban meters execution along several dimensions, and each category tracks the dominant dimension the lints in it are trying to save:

- **`StorageOperations`** → **Storage.** Every read or write on an instance, persistent, or temporary storage entry consumes a ledger entry access plus ledger I/O bytes. Storage is the single most expensive resource a typical contract can consume, so repeated, loop-invariant, or redundant storage access is the highest-priority class of lint.
- **`Compute`** → **CPU instructions.** Host function calls pay a dispatch overhead plus the work the host performs; signature verification, cross-contract calls, and other cryptographic or host-boundary work are among the most expensive calls available. Repeated, invariant, or unbounded CPU work belongs here.
- **`Memory`** → **RAM.** Needless clones, copies, and growth of SDK containers (`Bytes` / `Vec` / `Map`) allocate and copy guest linear memory. Memory is hard-capped per transaction but is *not* charged in the resource fee, so these lints are secondary to storage and CPU.
- **`EntryLifecycle`** → **Storage (ledger space rent / TTL).** Authorising, deploying, extending the TTL of, or removing ledger entries has a lifecycle cost — the rent payments that keep entries alive are priced from ledger size. Lints about *when* entries are created, extended, or allowed to expire belong here.
- **`SymbolOperations`** → **CPU.** `Symbol::new` constructs the symbol at runtime, which is metered; the `symbol_short!` macro produces a compile-time constant instead. Anything about how `Symbol` values are constructed or reused belongs here.

## Choosing a category for a new lint

Ask what the pattern wastes, in resource terms:

1. **Does the pattern touch Soroban storage?** A read, write, or existence check on `Storage` / `Instance` / `Persistent` / `Temporary` — directly, repeatedly, or in a loop — is `StorageOperations`.
2. **Is the waste CPU work?** A metered host call (`Ledger`, `Crypto`, `Prng`, `Events`, `Deployer`, ...), signature verification, a cross-contract call, or any computation repeated needlessly is `Compute`.
3. **Is the waste allocation or copying?** A needless `.clone()`, repeated concatenation, or growing an SDK container inside a loop is `Memory`.
4. **Does the pattern affect the lifecycle of ledger entries?** Authorisation, deployment, TTL extension, or removal — as opposed to the read/write itself — is `EntryLifecycle`.
5. **Does the pattern build or reuse `Symbol` values?** Runtime symbol construction that could be a compile-time constant is `SymbolOperations`.

When the answer is not obvious, look at the closest existing lint in `LINT_METADATA` and follow its category. The category is what the CLI uses to route diagnostics and `budget.toml` rows, so pick the *dominant* cost dimension of the pattern rather than a secondary one.

{% hint style="info" %}
Before writing a new lint, read [Scope: Clippy vs. soroban-cost-linter](scope_boundary.md) to confirm the pattern belongs here, and follow the [custom lint guide](custom_lint_guide.md) for the full workflow.
{% endhint %}
