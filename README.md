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

The linter hooks into the Rust compiler's AST to catch specific Soroban anti-patterns. Seven lints ship in `v0.1.1`:

*   **[`soroban_storage_in_loop`](docs/lints/soroban_storage_in_loop.md):** Flags storage read/write operations placed inside loop bodies, suggesting memory aggregation instead.
*   **[`redundant_env_clone`](docs/lints/redundant_env_clone.md):** Detects unnecessary `.clone()` calls on the Soroban `Env` object.
*   **[`unnecessary_host_function_call`](docs/lints/unnecessary_host_function_call.md):** Identifies host accessor calls (`Ledger`, `Crypto`, `Prng`, `Events`, `Deployer`, `Env::current_contract_address`) repeated inside a loop with unchanged inputs, which should be called once and bound to a local variable.
*   **[`storage_write_without_read`](docs/lints/storage_write_without_read.md):** Flags storage writes where the same key is never subsequently read.
*   **[`inefficient_bytes_concat`](docs/lints/inefficient_bytes_concat.md):** Detects repeated `Bytes` concatenation inside loops using `+`, which creates unnecessary per-iteration allocations.
*   **[`map_insert_in_loop`](docs/lints/map_insert_in_loop.md):** Flags `Map::insert` calls inside loop bodies.
*   **[`symbol_new_for_short_literal`](docs/lints/symbol_new_for_short_literal.md):** Flags `Symbol::new` calls with short literal arguments that could use `symbol_short!()`.

## How it Fits into Tollcraft

`soroban-cost-linter` is designed to be Stage 1 of your cost-awareness pipeline:

1.  **Linter (`soroban-cost-linter`):** Runs at compile-time (or via `cargo check`). Catches obvious, static structural flaws. 
2.  **Assert (`soroban-budget-assert`):** Runs at test-time. Simulates your cleanly-linted code against the network to measure actual execution costs based on real runtime inputs.

Both tools share configuration via a unified `budget.toml` file for thresholds and suppressions.

## Getting Started

### Prerequisites

Since `soroban-cost-linter` hooks directly into Rust's AST, it relies on [Dylint](https://github.com/trailofbits/dylint) to run dynamic library lints. The linter library requires Dylint version `^6.0.1`.

```bash
cargo install cargo-dylint dylint-link --version "^6.0.1"
```

### Installation

Add the linter to your Soroban workspace:

```bash
cargo install --git https://github.com/Tollcraft/soroban-cost-linter.git cargo-cost-lint

```

## Quick Start

```bash
# Install the tool
cargo install --git https://github.com/Tollcraft/soroban-cost-linter.git cargo-cost-lint

# Run it on your Soroban project
cargo cost-lint
```

## Usage

### Running the linter

From the root of your Soroban contract workspace:

```bash
cargo cost-lint
```

The linter will analyze all Rust source files and report any Soroban anti-patterns it finds. The output looks like this:

```text
error: storage operation inside a loop
  --> src/lib.rs:12:9
   |
LL |         env.storage().instance().set(&i, &1);
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: move storage operations out of the loop or accumulate mutations in memory first
   = note: `#[deny(soroban_storage_in_loop)]` on by default

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
   = help: pass Env by reference or value instead of cloning
   = note: `#[warn(redundant_env_clone)]` on by default
```

### Example: storage in a loop

**Bad** &mdash; a storage write inside a `for` loop writes on every iteration, driving up fees:

```rust
// ❌ Triggers: soroban_storage_in_loop
for item in items {
    env.storage().instance().set(&item, &1);
}
```

The linter flags this as:

```text
error: storage operation inside a loop
  --> src/lib.rs:4:9
   |
LL |         env.storage().instance().set(&item, &1);
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = help: move storage operations out of the loop or accumulate mutations in memory first
   = note: `#[deny(soroban_storage_in_loop)]` on by default
```

**Fix** &mdash; accumulate in memory, then write once:

```rust
// ✅ Fixed: single storage write after the loop
let mut aggregated = Vec::new();
for item in items {
    aggregated.push((item, 1u32));
}
for (key, val) in aggregated {
    env.storage().instance().set(&key, &val);
}
```

This works for all three storage types (`instance`, `persistent`, `temporary`) and all loop forms (`for`, `while`, `loop`).

### Example: redundant `Env` clone

**Bad:**

```rust
// ❌ Triggers: redundant_env_clone
let cloned = env.clone();
```

**Fix** &mdash; `Env` is cheap to copy by reference; just pass it through:

```rust
// ✅ Fixed
fn my_function(env: &Env) {
    // use &env
}
```

### Example: host function call in a loop

**Bad** &mdash; calling `env.ledger().sequence()` on every iteration is wasteful:

```rust
// ❌ Triggers: unnecessary_host_function_call
for _ in 0..10 {
    let _seq = env.ledger().sequence();
}
```

**Fix** &mdash; call once, bind to a local, reuse:

```rust
// ✅ Fixed
let seq = env.ledger().sequence();
for _ in 0..10 {
    // use seq
}
```

### Suppressing false positives

If a flagged pattern is intentional, suppress it with a standard Rust attribute:

```rust
#[allow(soroban_storage_in_loop)]
fn deliberate_storage_loop(env: Env) {
    for item in items {
        env.storage().instance().set(&item, &1);
    }
}
```

### Automatically fixing lints

`cargo-cost-lint` includes a `--fix` flag that automatically applies safe, machine-applicable suggestions for simple lints. For example, it can replace `Symbol::new(&env, "short")` with `symbol_short!("short")`:

```bash
# Check and auto-fix fixable lints
cargo cost-lint --fix
```

When `--fix` is passed, the tool applies all `MachineApplicable` suggestions in-place and writes the updated source files.

### Configuration (`budget.toml`)

You can define project-wide linting rules and severity levels in the same `budget.toml` file used by `soroban-budget-assert`. To apply that file, pass it explicitly with `--config` — see the next subsection. Without `--config`, the lints run at their declared default levels (`warn`):

```toml
[lints]
# Set to "warn", "deny", or "allow"
soroban_storage_in_loop = "deny"
redundant_env_clone = "warn"
unnecessary_host_function_call = "warn"
storage_write_without_read = "warn"
inefficient_bytes_concat = "warn"
map_insert_in_loop = "warn"

Inline diagnostics are supported through rust-analyzer's `check.overrideCommand` setting:

```json
{
  "rust-analyzer.check.overrideCommand": [
    "cargo",
    "cost-lint",
    "--all-diagnostics"
  ]
}
```

#### Pointing `cargo cost-lint` at a config — the `--config` flag

`cargo cost-lint` accepts a single `--config <PATH>` option. When the flag is omitted, **no config file is loaded** — the lints fall back to their rustc-declared default level (currently `warn` for all shipped lints). Today `--config` is the **only** way to apply a `budget.toml`: the tool does not auto-discover a workspace-root config.

To point the tool at a `budget.toml` that lives next to your code (or anywhere reachable), pass the path:

```bash
# relative path — resolved against the directory you run cargo cost-lint from
cargo cost-lint --config ./configs/strict.budget.toml

# absolute path — bypasses any workspace search
cargo cost-lint --config /abs/path/to/budget.toml
```

Unknown lint names or invalid levels fail validation identically whether the config comes from this flag or any other path.


## Contributing

We are actively looking for contributors in cost-model research, AST parsing, and lint specification.

1. Check the open issues to find tasks labeled `good first issue` or `help wanted`.
2. Fork the repository.
3. Ensure all Pull Requests target the `main` branch.
4. Pass all local tests before submitting.

See [CONTRIBUTING.md](CONTRIBUTING.md) for more detailed guidelines.

## Community

Join the discussion on our [Discord](https://discord.gg/5aprtMSyR).

## Maintainers

| Name | Role | Contact |
|---|---|---|
| [mallison031](https://github.com/mallison031) | Maintainer | [GitHub](https://github.com/mallison031) |
| Tollcraft Team | Core Maintainers | [Tollcraft on Telegram](https://t.me/+Gflo5jZStw1jMjE0) |

## Contributors

[![Contributors](https://contrib.rocks/image?repo=Tollcraft/soroban-cost-linter)](https://github.com/Tollcraft/soroban-cost-linter/graphs/contributors)
