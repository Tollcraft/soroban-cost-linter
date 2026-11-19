# Soroban Cost Rationale: Metered Resources and Dominant Operations

> This page explains *what* Soroban charges for and *which operations dominate* a contract's budget. It is the shared reference behind every lint in this repository. Written for contract developers, not compiler engineers.

---

## Budgets and Limits

Every Soroban transaction runs against a **per-transaction resource budget**. If execution exhausts the budget, the transaction fails — no partial state is committed. The budget is defined by network-wide limits (set by validator consensus) and reported in two dimensions[^1]:

| Resource dimension | Limit type | Charged in fee? |
|---|---|---|
| **CPU instructions** | Hard cap | Yes |
| **Memory (RAM)** | Hard cap | No (but enforced) |
| **Ledger entry reads** | Hard cap | Yes |
| **Ledger entry writes** | Hard cap | Yes |
| **Ledger I/O (bytes read/written)** | Hard cap | Yes |
| **Transaction size (bytes)** | Hard cap | Yes |

{% hint style="info" %}
Current per-transaction limits are published on the [Stellar Lab](https://lab.stellar.org/) under *Resource Limits & Fees*[^2]. These values change only through validator consensus.
{% endhint %}

---

## Metered Resources

Soroban's fee model is **multidimensional**: the resource fee is the sum of charges for several independent resource types[^1].

### 1. CPU Instructions

Every Wasm instruction the guest executes and every host function the contract calls is metered as CPU instructions. The host environment tracks more than 85 distinct **cost types** (`ContractCostType`[^3]), each with its own calibrated cost model of the form `y = a + bx`, where `x` is a runtime input size.

Key cost types that matter for linting:

| Cost type | What it models |
|---|---|
| `WasmInsnExec` | Baseline cost of one Wasm instruction |
| `DispatchHostFunction` | Overhead of crossing from Wasm into the host environment |
| `MemAlloc` / `MemCpy` / `MemCmp` | Memory-management operations |
| `VisitObject` | Accessing a host object from storage |

**The per-instruction cost is not uniform.** A `ComputeSha256Hash` call costs orders of magnitude more CPU than a `WasmInsnExec` — but every cost type is ultimately summed into the same instruction counter[^3].

### 2. Memory (RAM)

Memory is metered in bytes but **not included in the resource fee**. It is still subject to a hard per-transaction cap. If a contract allocates beyond the limit, execution is terminated.

For linting purposes, memory is a secondary concern: storage and CPU dominate the fee.

### 3. Storage: Ledger Entry Accesses and Ledger I/O

Storage operations are the **single most expensive resource** a typical Soroban contract can consume[^4]. The fee model charges in two sub-dimensions[^1]:

- **Ledger entry accesses** — each distinct storage key read or written counts as one access, regardless of its size.
- **Ledger I/O** — the total number of bytes read from or written to the ledger.

A single `env.storage().instance().set(&key, &val)` therefore incurs: one write access + `size_of(val)` write bytes + any CPU instructions for serialization. Repeating this inside a loop multiplies every dimension.

### 4. Transaction Size & Bandwidth

The size of the submitted transaction envelope (in bytes) is charged for network propagation and historical storage. This is rarely a linting concern — most structural anti-patterns exhaust the CPU or storage budget long before the bandwidth cap.

### 5. Events and Return Values

Events emitted by the contract and the top-level return value are included in transaction metadata and contribute to the resource fee. These costs are refundable: the network charges the declared maximum up front and refunds the unused portion after execution[^1].

### 6. Ledger Space Rent

Every ledger entry a contract creates or extends has a Time-To-Live (TTL). Extending TTL or increasing entry size incurs a **rent payment**, priced dynamically based on ledger size. This is a refundable fee component and depends on the state of the network, not just the contract's code structure.

#### Entry Lifecycle & Durability: Temporary vs. Persistent vs. Instance

How an entry behaves when its TTL hits zero depends entirely on which storage
type it lives in[^7]:

| Storage type | Behavior on TTL expiry | Cost | Intended for |
|---|---|---|---|
| `Temporary` | **Permanently deleted**, cannot be recovered | Cheapest | Time-bounded, *recreatable* data (oracles, signatures, allowances, cache) |
| `Persistent` | **Archived**, restorable (usually automatically) | Most expensive | Data that must survive (e.g. token balances) |
| `Instance` | **Archived** (shares the contract-instance TTL) | Most expensive | Small, fixed, contract-level data |

Temporary entries are never moved to archival storage: **when a `TemporaryEntry`
expires it is deleted forever and cannot be recovered**. The SDK documentation
states explicitly that this storage type is "best for entries that are only
relevant for short periods of time or for entries that can be **arbitrarily
recreated**", and warns that "it is unsafe to rely on the extensions to preserve
data — there is always a risk of losing temporary data."

By contrast, Persistent and Instance entries are archived on expiry and can be
restored, so they "behave as if stored forever". The consequence for contract
authors: a value that must remain available forever must go into `persistent()`
or `instance()` storage. Relying on a temporary entry to persist (e.g. reading
it back with `.unwrap()` after writing it) is relying on data that can quietly
vanish — this is exactly the anti-pattern the
[`temporary_storage_for_persistent_data`](lints/temporary_storage_for_persistent_data.md)
lint detects.

---

## What Dominates

For the patterns this linter catches, the resource hierarchy is:

| Rank | Operation | Primary resource consumed |
|---|---|---|
| 1 (most expensive) | **Storage writes** | Ledger entry writes + I/O bytes |
| 2 | **Storage reads** | Ledger entry reads + I/O bytes |
| 3 | **Host function calls** | CPU (dispatch + function work) |
| 4 | **Wasm arithmetic / control flow** | CPU (WasmInsnExec) |
| 5 (least expensive) | **Memory operations** | RAM (capped, not charged) |

**Storage writes dominate** because they consume four resources simultaneously: a ledger entry access, write I/O bytes, the serialization CPU cost, and (for new entries) space rent. A single storage write in a loop can cost more than the rest of the loop body combined.

**Host function calls** (e.g., `env.ledger().sequence()`) are cheaper than storage but still expensive relative to pure Wasm because they pay the `DispatchHostFunction` overhead plus whatever work the host performs[^3]. Calling a constant-returning host function inside a loop is pure waste.

---

## The Local-vs-Network Gap

One of the most surprising results from empirical measurement is how much local estimates can differ from real network costs. Measured on the example contract from the sibling repository `soroban-budget-assert`[^5]:

| Execution mode | CPU instructions | Gap vs. testnet |
|---|---|---|
| Raw Rust (native test) | 143,887 | **Under**estimates by ~**81%** |
| Local WASM (`register_contract_wasm`) | 901,816 | **Over**estimates by ~**19%** |
| Testnet simulation (`simulateTransaction`) | 756,678 | Ground truth |

{% hint style="warning" %}
Raw Rust test estimates are **unreliable** for budget decisions — they can miss real network cost by over 80%. Even WASM-mode local estimates can be off by double-digit percentages, and the direction (over vs. under) depends on the build profile.[^5]
{% endhint %}

**What this means for linting:** The linter catches *structural* anti-patterns that are expensive regardless of the local/network gap. A storage write in a loop is expensive everywhere. But the *magnitude* of savings from fixing it can only be known by running the compiled WASM against a network simulation — which is the purpose of the sibling project `soroban-budget-assert`.

---

## Per-Lint Resource Summary

Each lint in this repository targets a specific resource dimension. Every lint is assigned to one of the five [lint categories](lint_categories.md), and the table below shows the resource each one targets:

| Lint | Primary resource targeted | Why it matters |
|---|---|---|
| [`soroban_storage_in_loop`](lints/soroban_storage_in_loop.md) | **Storage** (ledger entry accesses + I/O bytes) | Storage writes are the #1 cost driver; multiplying them by loop count is the most expensive pattern this tool detects. |
| [`unnecessary_host_function_call`](lints/unnecessary_host_function_call.md) | **CPU** (host function dispatch) | Host calls are expensive relative to pure Wasm; repeating a constant-result call inside a loop wastes CPU. |
| [`signature_verification_in_loop`](lints/signature_verification_in_loop.md) | **CPU** (elliptic-curve cryptographic host functions) | Signature verification is one of the most expensive host functions available; verifying one at a time in a loop is a sign that a batch/aggregate scheme should be used instead. |
| [`redundant_env_clone`](lints/redundant_env_clone.md) | **CPU** (memory + dispatch overhead) | Cloning `Env` triggers `MemAlloc`/`MemCpy` and unnecessary object visits; the clone is never needed. |
| [`contract_call_in_loop`](lints/contract_call_in_loop.md) | **CPU** (cross-contract VM instantiation + dispatch) | Each `invoke_contract` call spins up a new VM context; repeating it per iteration multiplies that overhead by the loop count. |
| [`excessive_vec_capacity`](lints/excessive_vec_capacity.md) | **Memory** (guest linear memory, hard-capped) | A large hard-coded capacity is charged against the guest's memory cap the moment it's allocated, whether or not it's ever filled. |
| [`extend_ttl_in_loop`](lints/extend_ttl_in_loop.md) | **Storage** (ledger space rent) | `extend_ttl` is a metered host call that also pays rent; calling it once per iteration multiplies both costs by the loop count instead of batching the extension. |
| [`temporary_storage_for_persistent_data`](lints/temporary_storage_for_persistent_data.md) | **Entry Lifecycle** (temporary-storage durability) | A temporary entry is permanently deleted when its TTL expires; reading it back with `.unwrap()`/`.expect()` assumes it persists and panics (or worse, compounds) once the entry is gone, wasting every metered call before the failure. |
| [`crypto_hash_of_constant`](lints/crypto_hash_of_constant.md) | **CPU** (Wasm instruction execution + host function dispatch) | Hashing a value fixed at compile time repeats the same host-function work on every invocation; the digest can be precomputed once and embedded. |
| [`host_in_loop`](lints/host_in_loop.md) | **CPU** (Wasm instruction execution + host function dispatch) | Obtaining a Host handle crosses the VM boundary; doing it per iteration pays that cost every time for a handle that does not change. |
| [`string_concat_in_loop`](lints/string_concat_in_loop.md) | **Memory** (guest linear memory, hard-capped) | Soroban String is immutable, so each concatenation allocates and copies the whole buffer; in a loop that is O(n^2) byte copies. |
| [`redundant_require_auth`](lints/redundant_require_auth.md) | **CPU** (Wasm instruction execution + host function dispatch) | require_auth walks the authorization tree and verifies signatures; calling it twice on the same address costs twice and proves nothing new. |
| [`unnecessary_host_function_call_legacy`](lints/unnecessary_host_function_call_legacy.md) | **CPU** (host function dispatch, VM boundary crossing) | Retained so existing allow/deny configuration keeps working; the host call is still charged on every invocation. |
| [`soroban_redundant_storage_read`](lints/soroban_redundant_storage_read.md) | **Storage** (ledger entry accesses + I/O bytes) | Repeatedly reading the same storage entry without intervening writes wastes ledger IO and CPU. |
| [`storage_write_without_read`](lints/storage_write_without_read.md) | **Storage** (ledger entry accesses + I/O bytes) | Blind writes can overwrite state accidentally and lack update guards. |
| [`discarded_storage_read`](lints/discarded_storage_read.md) | **Storage** (ledger entry accesses + I/O bytes) | Storage reads are among the most expensive operations in Soroban; reading data without using it wastes ledger bandwidth and gas with zero behavioral purpose. |
| [`instance_storage_for_unbounded_data`](lints/instance_storage_for_unbounded_data.md) | **Storage** (ledger entry accesses + I/O bytes) | Instance storage has a strict 64KB footprint limit and shares contract TTL. |
| [`persistent_read_without_ttl_extension`](lints/persistent_read_without_ttl_extension.md) | **Storage** (ledger entry accesses + I/O bytes) | Persistent entries expire if their TTL is not extended, risking archival. |
| [`loop_invariant_storage_access`](lints/loop_invariant_storage_access.md) | **Storage** (ledger entry accesses + I/O bytes) | Invariants should be hoisted outside the loop. |
| [`storage_key_construction_in_loop`](lints/storage_key_construction_in_loop.md) | **Storage** (ledger entry accesses + I/O bytes) | Key construction inside loops wastes CPU. |
| [`bytes_append_in_loop`](lints/bytes_append_in_loop.md) | **Memory** (guest linear memory, hard-capped) | Reallocating memory inside loops is inefficient. |
| [`unbounded_input_loop`](lints/unbounded_input_loop.md) | **CPU** (Wasm instruction execution + host function dispatch) | Loops controlled by untrusted input can cause excessive execution cost. |
| [`unnecessary_string_to_bytes`](lints/unnecessary_string_to_bytes.md) | **Memory** (guest linear memory, hard-capped) | Unnecessary string-to-bytes conversions waste CPU cycles. |
| [`map_insert_in_loop`](lints/map_insert_in_loop.md) | **CPU** (Wasm instruction execution + host function dispatch) | Map insertions inside loops can be expensive. |
| [`inefficient_bytes_concat`](lints/inefficient_bytes_concat.md) | **Memory** (guest linear memory, hard-capped) | Concatenating bytes inefficiently leads to high memory overhead. |
| [`formatted_panic_payload`](lints/formatted_panic_payload.md) | **CPU** (Wasm instruction execution + host function dispatch) | Formatted panic strings consume unnecessary memory and CPU. |
| [`linear_scan_in_loop`](lints/linear_scan_in_loop.md) | **CPU** (Wasm instruction execution + host function dispatch) | Linear scans inside loops degrade algorithmic complexity. |
| [`require_auth_in_loop`](lints/require_auth_in_loop.md) | **CPU** (cryptographic operations, signature verification) | Authorization checks inside loops repeat expensive signature validations. |
| [`symbol_key_boundary`](lints/symbol_key_boundary.md) | **Storage** (ledger entry accesses + I/O bytes) | Ensure symbol keys respect length limits and conventions. |
| [`symbol_key_enum_storage`](lints/symbol_key_enum_storage.md) | **Storage** (ledger entry accesses + I/O bytes) | Optimizes enum storage keys. |
| [`symbol_key_event_topics`](lints/symbol_key_event_topics.md) | **CPU** (host function dispatch, VM boundary crossing) | Optimizes event topic symbol keys. |
| [`symbol_new_for_short_literal`](lints/symbol_new_for_short_literal.md) | **CPU** (Wasm instruction execution + host function dispatch) | Use symbol_short! macro instead of Symbol::new for short literals. |
| [`unbounded_recursion`](lints/unbounded_recursion.md) | **CPU** (Wasm instruction execution + host function dispatch) | Recursion without bounds can overflow stack and exhaust resources. |
| [`unwrap_on_storage_get`](lints/unwrap_on_storage_get.md) | **Storage** (ledger entry accesses + I/O bytes) | Unwrapping optional storage gets can cause unexpected contract panics when keys are absent. |
| [`vec_where_slice_could_be_used`](lints/vec_where_slice_could_be_used.md) | **Memory** (guest linear memory, hard-capped) | Slices avoid unnecessary heap allocations. |
| [`soroban_inefficient_bytes_concat`](lints/soroban_inefficient_bytes_concat.md) | **Memory** (guest linear memory, hard-capped) | Inefficient bytes concatenation in Soroban environment. |
| [`u128_where_u64_suffices`](lints/u128_where_u64_suffices.md) | **CPU** (Wasm instruction execution + host function dispatch) | wasm32 lacks native 128-bit integer instructions; emulating them is very slow. |
| [`float_arithmetic_in_contract`](lints/float_arithmetic_in_contract.md) | **CPU** (Wasm instruction execution + host function dispatch) | wasm32 has no hardware floating point; every f32/f64 operation compiles to a soft-float routine costing tens to hundreds of wasm instructions. Floats are also non-deterministic across rounding paths. |
| [`duplicate_storage_key_construction`](lints/duplicate_storage_key_construction.md) | **Storage** (ledger entry accesses + I/O bytes) | Rebuilding the same key across functions wastes the symbol-construction host call and introduces independent chances to typo the key into a silent, undebuggable state bug. |
| [`option_wrapping_in_storage`](lints/option_wrapping_in_storage.md) | **Storage** (ledger entry accesses + I/O bytes) | Storage already models absence — a missing key returns None. Storing Option<T> creates a redundant three-state model. |
| [`ledger_context_read_in_loop`](lints/ledger_context_read_in_loop.md) | **CPU** (Wasm instruction execution + host function dispatch) | Ledger context values are invariant during a single invocation; reading them in a loop performs repeated host calls for the same value. |
| [`large_constant_array`](lints/large_constant_array.md) | **Memory** (guest linear memory, hard-capped) | Embedding large constant arrays in the contract code increases Wasm binary size, which drives up deployment costs and loads. Consider using host-managed Bytes or storing large data in persistent storage. |

---|---|---|
| [`soroban_storage_in_loop`](lints/soroban_storage_in_loop.md) | **Storage** (ledger entry accesses + I/O bytes) | Storage writes are the #1 cost driver; multiplying them by loop count is the most expensive pattern this tool detects. |
| [`unnecessary_host_function_call`](lints/unnecessary_host_function_call.md) | **CPU** (host function dispatch) | Host calls are expensive relative to pure Wasm; repeating a constant-result call inside a loop wastes CPU. |
| [`signature_verification_in_loop`](lints/signature_verification_in_loop.md) | **CPU** (elliptic-curve cryptographic host functions) | Signature verification is one of the most expensive host functions available; verifying one at a time in a loop is a sign that a batch/aggregate scheme should be used instead. |
| [`redundant_env_clone`](lints/redundant_env_clone.md) | **CPU** (memory + dispatch overhead) | Cloning `Env` triggers `MemAlloc`/`MemCpy` and unnecessary object visits; the clone is never needed. |
| [`contract_call_in_loop`](lints/contract_call_in_loop.md) | **CPU** (cross-contract VM instantiation + dispatch) | Each `invoke_contract` call spins up a new VM context; repeating it per iteration multiplies that overhead by the loop count. |
| [`excessive_vec_capacity`](lints/excessive_vec_capacity.md) | **Memory** (guest linear memory, hard-capped) | A large hard-coded capacity is charged against the guest's memory cap the moment it's allocated, whether or not it's ever filled. |
| [`extend_ttl_in_loop`](lints/extend_ttl_in_loop.md) | **Storage** (ledger space rent) | `extend_ttl` is a metered host call that also pays rent; calling it once per iteration multiplies both costs by the loop count instead of batching the extension. |
| [`temporary_storage_for_persistent_data`](lints/temporary_storage_for_persistent_data.md) | **Entry Lifecycle** (temporary-storage durability) | A temporary entry is permanently deleted when its TTL expires; reading it back with `.unwrap()`/`.expect()` assumes it persists and panics (or worse, compounds) once the entry is gone, wasting every metered call before the failure. |

---

## What We Don't Yet Know

- **Exact per-instruction CPU costs for every `ContractCostType`** — the calibrated model parameters (`a`, `b` for each cost type) are set by network consensus and are not published in developer-facing documentation. They can be inspected in the `rs-soroban-env` source repository[^6].
- **Decomposed storage costs** — the ratio of "ledger entry access fee" to "I/O byte fee" is not specified independently. The total storage fee is what matters for linting, but measuring the split requires network simulation.

{% hint style="info" %}
Local measurements are available in the [`cost_benchmarks`](https://github.com/Tollcraft/soroban-cost-linter/tree/main/cost_benchmarks) crate. Run `cargo test -- --nocapture` to see before/after budget deltas for each lint pattern on `Env::default()`. These numbers are **directional** (they show relative savings) but are subject to the [Local-vs-Network Gap](#the-local-vs-network-gap) described above.
{% endhint %}

---

## References

[^1]: Stellar Development Foundation, *"Understanding Fees, Resource Limits, and Metering"*, Stellar Docs. [https://developers.stellar.org/docs/learn/fundamentals/fees-resource-limits-metering](https://developers.stellar.org/docs/learn/fundamentals/fees-resource-limits-metering)

[^2]: Stellar Lab — *Resource Limits & Fees*. [https://lab.stellar.org/](https://lab.stellar.org/)

[^3]: Stellar XDR Specification, *`ContractCostType` enum* (86 variants). [https://docs.rs/stellar-xdr/latest/stellar_xdr/enum.ContractCostType.html](https://docs.rs/stellar-xdr/latest/stellar_xdr/enum.ContractCostType.html)

[^4]: `soroban-cost-linter` README — *"Storage operations in Soroban are the most expensive resource."* [https://github.com/Tollcraft/soroban-cost-linter](https://github.com/Tollcraft/soroban-cost-linter)

[^5]: `soroban-budget-assert` — *Protocol Mechanics: The measured gap*. [https://github.com/Tollcraft/soroban-budget-assert/blob/main/docs/src/mechanics.md](https://github.com/Tollcraft/soroban-budget-assert/blob/main/docs/src/mechanics.md)

[^6]: Stellar `rs-soroban-env` — host-side cost model definitions. [https://github.com/stellar/rs-soroban-env](https://github.com/stellar/rs-soroban-env)

[^7]: Soroban SDK `Storage` documentation and Stellar *State Archival* guide — storage-type durability semantics (`Temporary` deleted forever, `Persistent`/`Instance` archived and restorable). [https://docs.rs/soroban-sdk/latest/soroban_sdk/storage/struct.Storage.html](https://docs.rs/soroban-sdk/latest/soroban_sdk/storage/struct.Storage.html) and [https://developers.stellar.org/docs/learn/fundamentals/contract-development/storage/state-archival](https://developers.stellar.org/docs/learn/fundamentals/contract-development/storage/state-archival)
