# soroban-cost-linter

> Part of the **[`Tollcraft`](https://github.com/Tollcraft)** initiative, funded by Drips Wave 7.

`soroban-cost-linter` is a static analysis tool for Stellar smart contract developers. It analyzes your Rust code before compilation to detect input-independent, structurally expensive patterns that would unnecessarily drive up your Soroban resource metering and network fees.

This tool acts as the preventative shield in the Tollcraft two-tiered cost pipeline, pairing conceptually with our runtime test harness, [`soroban-budget-assert`](https://github.com/Tollcraft/soroban-budget-assert).

## The Problem

Soroban charges for CPU instructions, memory allocations, and storage operations. While testing your contract against the network is the only way to measure *input-dependent* costs (like unbounded loops or dynamic vector sizing), many expensive mistakes are structurally obvious without ever running the code. 

Writing `env.storage().instance().set()` inside a `for` loop is mathematically guaranteed to be expensive. `soroban-cost-linter` catches these structural anti-patterns directly in your IDE or CI/CD pipeline before they make it to testnet.

## Features (MVP Scope)

Our initial research and architecture focus on hooking into the Rust compiler's AST to catch specific Soroban anti-patterns:

*   **`soroban_storage_in_loop` (WIP):** Flags storage read/write operations placed inside loop bodies, suggesting memory aggregation instead.
*   **`redundant_env_clone` (WIP):** Detects unnecessary `.clone()` calls on the Soroban `Env` object.
*   **`unnecessary_host_function_call` (WIP):** Identifies redundant calls to host functions (like fetching the ledger sequence) that should be called once and bound to a local variable.

## How it Fits into Tollcraft

`soroban-cost-linter` is designed to be Stage 1 of your cost-awareness pipeline:
1.  **Linter (`soroban-cost-linter`):** Runs at compile-time (or via `cargo check`). Catches obvious, static structural flaws. 
2.  **Assert (`soroban-budget-assert`):** Runs at test-time. Simulates your cleanly-linted code against the network to measure actual execution costs based on real runtime inputs.

Both tools share configuration via a unified `budget.toml` file for thresholds and suppressions.

## Getting Started

*(Instructions for installing the Dylint-based cargo wrapper will be added here once the MVP binary is published.)*

```bash
# Planned usage:
cargo cost-lint
