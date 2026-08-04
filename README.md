<div align="center">
  <h1>soroban-cost-linter</h1>
  <p><strong>The static analysis shield for Soroban smart contracts</strong></p>
  <p>
    <img src="https://img.shields.io/github/actions/workflow/status/Tollcraft/soroban-cost-linter/lint.yml?branch=main" alt="CI Status" />
    <img src="https://img.shields.io/badge/License-Apache%202.0-blue.svg" alt="License" />
  </p>
  <p>
    <a href="https://tollcraft.gitbook.io/docs"><strong>Documentation</strong></a> ·
    <a href="https://asciinema.org/a/1DpqHMqqOOXoZzMI"><strong>Demo</strong></a>
  </p>
</div>

> Part of the **[`Tollcraft`](https://github.com/Tollcraft)** initiative.

`soroban-cost-linter` is a static analysis tool for Stellar smart contract developers. It analyzes your Rust code before compilation to detect input-independent, structurally expensive patterns that would unnecessarily drive up your Soroban resource metering and network fees.

This tool acts as the preventative shield in the Tollcraft two-tiered cost pipeline, pairing conceptually with our runtime test harness, [`soroban-budget-assert`](https://github.com/Tollcraft/soroban-budget-assert).

## The Problem

Soroban charges for CPU instructions, memory allocations, and storage operations. While testing your contract against the network is the only way to measure *input-dependent* costs (like unbounded loops or dynamic vector sizing), many expensive mistakes are structurally obvious without ever running the code. 

Writing `env.storage().instance().set()` inside a `for` loop is mathematically guaranteed to be expensive. `soroban-cost-linter` catches these structural anti-patterns directly in your [editor](docs/integration.md#editor--ide-integration) or [CI/CD pipeline](docs/integration.md#github-actions) before they make it to testnet.

## Features

The linter hooks into the Rust compiler's AST to catch specific Soroban anti-patterns. Thirteen lints ship in `v0.1.1`:

*   **[`soroban_storage_in_loop`](docs/lints/soroban_storage_in_loop.md):** Flags storage read/write operations placed inside loop bodies, suggesting memory aggregation instead.
*   **[`redundant_env_clone`](docs/lints/redundant_env_clone.md):** Detects unnecessary `.clone()` calls on the Soroban `Env` object.
*   **[`unnecessary_host_function_call`](docs/lints/unnecessary_host_function_call.md):** Identifies host accessor calls (`Ledger`, `Crypto`, `Prng`, `Events`, `Deployer`, `Env::current_contract_address`) repeated inside a loop with unchanged inputs, which should be called once and bound to a local variable.
*   **[`storage_write_without_read`](docs/lints/storage_write_without_read.md):** Flags storage writes where the same key is never subsequently read.
*   **[`inefficient_bytes_concat`](docs/lints/inefficient_bytes_concat.md):** Detects repeated `Bytes` concatenation inside loops using `+`, which creates unnecessary per-iteration allocations.
*   **[`map_insert_in_loop`](docs/lints/map_insert_in_loop.md):** Flags `Map::insert` calls inside loop bodies.
*   **[`symbol_new_for_short_literal`](docs/lints/symbol_new_for_short_literal.md):** Flags `Symbol::new` calls with short literal arguments that could use `symbol_short!()`.
*   **[`bytes_append_in_loop`](docs/lints/bytes_append_in_loop.md):** Flags repeatedly growing SDK containers (`Bytes::append`, `Vec::push_back`, `Map::insert`) inside loops, suggesting native accumulation first.
*   **[`signature_verification_in_loop`](docs/lints/signature_verification_in_loop.md):** Flags `env.crypto().ed25519_verify`/`secp256k1_recover`/`secp256r1_verify` calls made inside loop bodies, suggesting batch/aggregate verification instead.
*   **[`vec_where_slice_could_be_used`](docs/lints/vec_where_slice_could_be_used.md):** Flags `soroban_sdk::Vec` passed by value where a native Rust `&[T]` slice would be sufficient for read-only access.
*   **[`extend_ttl_in_loop`](docs/lints/extend_ttl_in_loop.md):** Flags `extend_ttl` calls on instance/persistent/temporary storage made inside loop bodies, suggesting batching the TTL extension instead of refreshing per-entry per-iteration.
*   **[`instance_storage_for_unbounded_data`](docs/lints/instance_storage_for_unbounded_data.md):** Flags `env.storage().instance().set(...)` calls where the value is an unbounded `Vec`/`Map`/`Bytes`, since instance storage is re-read and rewritten in full on every contract invocation.
*   **[`formatted_panic_payload`](docs/lints/formatted_panic_payload.md):** Flags `format!`, a formatted `panic!`, or `.expect(&format!(..))`, all of which pull `core::fmt` into the contract in place of a cheap `panic_with_error!` + `#[contracterror]`.

## How it Fits into Tollcraft

`soroban-cost-linter` is designed to be Stage 1 of your cost-awareness pipeline:

1.  **Linter (`soroban-cost-linter`):** Runs at compile-time (or via `cargo check`). Catches obvious, static structural flaws. 
2.  **Assert (`soroban-budget-assert`):** Runs at test-time. Simulates your cleanly-linted code against the network to measure actual execution costs based on real runtime inputs.

Both tools share configuration via a unified `budget.toml` file for thresholds and suppressions.

## Getting Started

### Recommended: Dev Container

The fastest way to get a working environment is the pre-built container image, which ships with
the exact nightly toolchain, compiler components, and Dylint binaries installed — no manual setup required.

```bash
docker pull ghcr.io/Tollcraft/soroban-cost-linter:latest
docker run --rm -it -v "$(pwd)":/workspace ghcr.io/Tollcraft/soroban-cost-linter:latest bash
# Inside the container:
cargo test --workspace
```

VS Code / GitHub Codespaces users can open the repo and choose **"Reopen in Container"** — the
`.devcontainer/devcontainer.json` handles everything automatically.

See [CONTRIBUTING.md](CONTRIBUTING.md) for full setup details, including a manual local setup path.

### Prerequisites

> **Windows users:** the project CI runs on Ubuntu. For the smoothest setup,
> prefer **WSL2 with Ubuntu** — see
> [docs/windows_setup.md](docs/windows_setup.md). Native-PowerShell install is
> covered in the same page; **Visual Studio Build Tools is required** because
> the MSVC `rustc` toolchain needs `link.exe` (which Build Tools provides).

Since `soroban-cost-linter` hooks directly into Rust's AST, it relies on [Dylint](https://github.com/trailofbits/dylint) to run dynamic library lints. The linter library requires Dylint version `^6.0.1`.

1. **Install the pinned nightly toolchain** — see the [`rust-toolchain`](rust-toolchain) file for the exact channel (as of this writing, the CI uses `nightly-2026-04-16`).

   ```bash
   rustup toolchain install <channel-from-rust-toolchain>
   ```

2. **Install Dylint** — the linter relies on [Dylint](https://github.com/trailofbits/dylint) version `^6.0.1` to run dynamic library lints:

   ```bash
   cargo install cargo-dylint dylint-link --version "^6.0.1"
   ```

> **Windows:** Install via PowerShell after setting up Rust through [rustup](https://rustup.rs/). The command is identical. Make sure the nightly toolchain with `rustc-dev` and `llvm-tools-preview` components is installed (`rustup toolchain install nightly --component rustc-dev llvm-tools-preview`).

### Installation

Add the linter to your Soroban workspace. **Ensure you are using the pinned nightly toolchain** (see [Prerequisites](#prerequisites)) when building:

```bash
cargo +<channel-from-rust-toolchain> install --git https://github.com/Tollcraft/soroban-cost-linter.git cargo-cost-lint
```

> **Why is the nightly required?** The lint library links against `rustc_private`, which is only available on nightly compilers. A different nightly version may produce linker errors due to ABI mismatches.

## Quick Start

1. Complete the [Prerequisites](#prerequisites) (nightly toolchain + Dylint).
2. Install the linter using the pinned nightly:

   ```bash
   cargo +<channel-from-rust-toolchain> install --git https://github.com/Tollcraft/soroban-cost-linter.git cargo-cost-lint
   ```
3. Run it on your Soroban project:

   ```bash
   cargo cost-lint
   ```

## Usage

### CLI Flags

| Flag | Description |
|------|-------------|
| `--config <PATH>` | Path to `budget.toml` for lint-level overrides |
| `--format <text\|json>` | Output format (default: `text`) |
| `--list-lints` | Print every registered lint with its default level and one-line description, then exit |
| `--explain <LINT>` | Print the full documentation for a specific lint (what it does, why it's expensive, suggested fix) and exit |
| `--version` | Print the crate version and exit |

### Running the linter

From the root of your Soroban contract workspace:

```bash
cargo cost-lint
```

To inspect the machine-readable lint inventory that the CLI emits, run:

```bash
cargo cost-lint --list-lints --format json
```

The output is a versioned JSON object with the lint name, default level, description, category, and documentation URL for every registered lint.

The linter will analyze all Rust source files and report any Soroban anti-patterns it finds. The output looks like this:

```text
error: storage operation inside a loop
  --> src/lib.rs:12:9
   |
LL |         env.storage().instance().set(&i, &1);
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: move storage operations out of the loop or accumulate mutations in memory first
   = note: `#[warn(soroban_storage_in_loop)]` on by default
```

#### Output format

Use `--format` to choose the output format:

| Format  | Description                                                  |
| ------- | ------------------------------------------------------------ |
| `text`  | Human-readable console output (default)                      |
| `json`  | One JSON object per line, suitable for programmatic parsing  |
| `sarif` | SARIF v2.1.0 output, compatible with GitHub Code Scanning   |

Example — generate SARIF output for GitHub Advanced Security:

```bash
cargo cost-lint --format sarif > results.sarif
```

The SARIF file can then be uploaded to GitHub or integrated into your CI pipeline to annotate PR diffs with line-specific warnings.

```text
warning: storage operation inside a loop
  --> src/lib.rs:12:9
   |
LL |         env.storage().instance().set(&i, &1);
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: move storage operations out of the loop or accumulate mutations in memory first
   = note: `#[warn(soroban_storage_in_loop)]` on by default

warning: unnecessary host function call inside loop
  --> src/lib.rs:20:20
   |
LL |         let _seq = env.ledger().sequence();
   |                    ^^^^^^^^^^^^^^^^^^^^^^^
   = help: call this function outside the loop and reuse the result
   = note: `#[warn(unnecessary_host_function_call)]` on by default

warning: redundant clone on Env object
  --> src/lib.rs:30:19
   |
LL |     let _cloned = env.clone();
   |                   ^^^^^^^^^^^
   |
   = help: pass `Env` by reference or value instead of cloning
   = note: `#[warn(redundant_env_clone)]` on by default

lint summary:
  redundant_env_clone: 1
  soroban_storage_in_loop: 3
  unnecessary_host_function_call: 2
total: 6
```

For detailed examples of each lint and instructions on suppressing false positives, see the [`docs/lints/`](docs/lints) directory.

For instructions on CI/CD integration, configuring the `budget.toml` file, and setting a maximum warnings threshold, see the [Integration Guide](docs/integration.md).


## Contributing

We are actively looking for contributors in cost-model research, AST parsing, and lint specification.

1. Check the open issues to find tasks labeled `good first issue` or `help wanted`.
2. Fork the repository.
3. Ensure all Pull Requests target the `main` branch.
4. Pass all local tests before submitting.

See [CONTRIBUTING.md](CONTRIBUTING.md) for more detailed guidelines.
**Windows contributors**, start with [docs/windows_setup.md](docs/windows_setup.md) for WSL2 and native-PowerShell setup instructions.

Release history is documented in [CHANGELOG.md](CHANGELOG.md).

## Community

Join the discussion on our [Discord](https://discord.gg/5aprtMSyR).

## Maintainers

| Name | Role | Contact |
|---|---|---|
| [mallison031](https://github.com/mallison031) | Maintainer | [GitHub](https://github.com/mallison031) |
| Tollcraft Team | Core Maintainers | [Tollcraft on Telegram](https://t.me/+Gflo5jZStw1jMjE0) |

## Contributors

[![Contributors](https://contrib.rocks/image?repo=Tollcraft/soroban-cost-linter)](https://github.com/Tollcraft/soroban-cost-linter/graphs/contributors)


<!-- [`soroban-budget-assert`](https://github.com/Tollcraft/soroban-budget-assert). -->