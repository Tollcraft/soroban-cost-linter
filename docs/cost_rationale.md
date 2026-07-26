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

Each lint in this repository targets a specific resource dimension:

| Lint | Primary resource targeted | Why it matters |
|---|---|---|
| [`soroban_storage_in_loop`](lints/soroban_storage_in_loop.md) | **Storage** (ledger entry accesses + I/O bytes) | Storage writes are the #1 cost driver; multiplying them by loop count is the most expensive pattern this tool detects. |
| [`unnecessary_host_function_call`](lints/unnecessary_host_function_call.md) | **CPU** (host function dispatch) | Host calls are expensive relative to pure Wasm; repeating a constant-result call inside a loop wastes CPU. |
| [`redundant_env_clone`](lints/redundant_env_clone.md) | **CPU** (memory + dispatch overhead) | Cloning `Env` triggers `MemAlloc`/`MemCpy` and unnecessary object visits; the clone is never needed. |

---

## What We Don't Yet Know

- **Exact per-instruction CPU costs for every `ContractCostType`** — the calibrated model parameters (`a`, `b` for each cost type) are set by network consensus and are not published in developer-facing documentation. They can be inspected in the `rs-soroban-env` source repository[^6].
- **Decomposed storage costs** — the ratio of "ledger entry access fee" to "I/O byte fee" is not specified independently. The total storage fee is what matters for linting, but measuring the split requires network simulation.
- **Measured figures for each lint** — issue [#10] tracks adding specific measured savings (in CPU instructions and storage bytes) to individual lint pages. That work depends on measurements that do not yet exist.

{% hint style="info" %}
An honest "not yet measured" is more useful than a guess. When specific numbers become available from `soroban-budget-assert` measurements, they belong on the individual lint pages, not here.
{% endhint %}

---

## References

[^1]: Stellar Development Foundation, *"Understanding Fees, Resource Limits, and Metering"*, Stellar Docs. [https://developers.stellar.org/docs/learn/fundamentals/fees-resource-limits-metering](https://developers.stellar.org/docs/learn/fundamentals/fees-resource-limits-metering)

[^2]: Stellar Lab — *Resource Limits & Fees*. [https://lab.stellar.org/](https://lab.stellar.org/)

[^3]: Stellar XDR Specification, *`ContractCostType` enum* (86 variants). [https://docs.rs/stellar-xdr/latest/stellar_xdr/enum.ContractCostType.html](https://docs.rs/stellar-xdr/latest/stellar_xdr/enum.ContractCostType.html)

[^4]: `soroban-cost-linter` README — *"Storage operations in Soroban are the most expensive resource."* [https://github.com/Tollcraft/soroban-cost-linter](https://github.com/Tollcraft/soroban-cost-linter)

[^5]: `soroban-budget-assert` — *Protocol Mechanics: The measured gap*. [https://github.com/Tollcraft/soroban-budget-assert/blob/main/docs/src/mechanics.md](https://github.com/Tollcraft/soroban-budget-assert/blob/main/docs/src/mechanics.md)

[^6]: Stellar `rs-soroban-env` — host-side cost model definitions. [https://github.com/stellar/rs-soroban-env](https://github.com/stellar/rs-soroban-env)
