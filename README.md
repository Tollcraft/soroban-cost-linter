<div align="center">
  <h1>soroban-cost-linter</h1>
  <p><strong>The static analysis shield for Soroban smart contracts</strong></p>
  <p>
    <img src="https://img.shields.io/github/actions/workflow/status/Tollcraft/soroban-cost-linter/lint.yml?branch=main" alt="CI Status" />
    <img src="https://img.shields.io/crates/v/soroban-cost-linter.svg" alt="Crates.io" />
    <img src="https://img.shields.io/badge/License-Apache%202.0-blue.svg" alt="License" />
  </p>
</div>

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

### Prerequisites

Since `soroban-cost-linter` hooks directly into Rust's AST, it relies on [Dylint](https://github.com/trailofbits/dylint) to run dynamic library lints.

```bash
cargo install cargo-dylint dylint-link

```

### Installation

Add the linter to your Soroban workspace:

```bash
cargo install --git [https://github.com/your-org/soroban-cost-linter.git](https://github.com/your-org/soroban-cost-linter.git)

```

### Usage

Run the linter across your entire workspace:

```bash
cargo cost-lint

```

To suppress a false positive or an intentionally expensive operation, standard Rust attributes are fully supported. Place this directly above the flagged function or block:

```rust
#[allow(soroban_cost::storage_in_loop)]
for item in items {
    // Deliberate storage loop
}

```

### Configuration (`budget.toml`)

You can define project-wide linting rules and severity levels in the same `budget.toml` file used by `soroban-budget-assert`. Place this in your workspace root:

```toml
[lints]
# Set to "warn", "deny", or "allow"
soroban_storage_in_loop = "deny"
redundant_env_clone = "warn"
unnecessary_host_function_call = "warn"

```

## Contributing

We are actively looking for contributors in cost-model research, AST parsing, and lint specification.

1. Check the open issues to find tasks labeled `good first issue` or `help wanted`.
2. Fork the repository.
3. Ensure all Pull Requests target the `v1` branch.
4. Pass all local tests before submitting.

See [CONTRIBUTING.md](CONTRIBUTING.md) for more detailed guidelines.

## Community

Join the discussion on our [Discord](https://discord.gg/tollcraft).

## Maintainers

| Name | Role | Contact |
|---|---|---|
| Tollcraft Team | Core Maintainers | [@Tollcraft on Telegram](https://t.me/Tollcraft) |

## Contributors

[![Contributors](https://contrib.rocks/image?repo=Tollcraft/soroban-cost-linter)](https://github.com/Tollcraft/soroban-cost-linter/graphs/contributors)
