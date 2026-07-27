# Contributing to `soroban-cost-linter`

First off, thank you for considering contributing to `soroban-cost-linter`!

## How to Contribute

### 1. Understanding the Architecture

This tool leverages Dylint to hook into the Rust compiler's AST and High-Level Intermediate Representation (HIR). Familiarity with `rustc` internals (like `rustc_hir`, `rustc_middle`) and `clippy` source code is highly beneficial.

### 2. Setting up Locally

1. Install Dylint:

   ```bash
   cargo install cargo-dylint dylint-link --version "^6.0.1"
   ```

2. Clone the repository and build:

   ```bash
   cargo build
   ```

3. Run tests:

   ```bash
   cargo test
   ```

### 3. Adding a New Lint
- Read the [Scope: Clippy vs. soroban-cost-linter](../docs/scope_boundary.md) guide first. If a pattern is already covered by a Clippy lint and the Soroban cost story does not change the analysis, do not duplicate it here.
- Find a structural anti-pattern in Soroban that is input-independent and costly, and that is **not** already covered by a Clippy lint with the same cost-relevant semantics.
- Assign the lint to one of the defined cost categories (`StorageOperations`, `Compute`, `Memory`, or `EntryLifecycle`) and add it to the `LINT_METADATA` registry in `soroban_cost_lints/src/lib.rs`.
- Write a failing test case in the `ui` tests directory.
- Implement the lint using the `dylint` framework, checking the AST or HIR for the specific pattern.
- Update the documentation and `README.md`, ensuring the new lint is placed under the correct category header.

### Lint Naming Convention

Lint names are part of the project's public API. They appear in `#[allow(...)]`, `#[warn(...)]`, `#[deny(...)]`, and `budget.toml`, so renaming a shipped lint is a breaking change and should be avoided.

When adding new lints:

- Use lowercase `snake_case`.
- Prefer names that describe the code pattern being detected rather than expressing a judgement about the code.
- Use consistent suffixes such as `_in_loop` for loop-specific patterns.
- Avoid a `soroban_` prefix unless it is needed to avoid ambiguity with another well-known lint name.

Examples:

| Preferred | Avoid |
| ---------- | ----- |
| `storage_in_loop` | `soroban_storage_in_loop` |
| `env_clone` | `redundant_env_clone` |
| `host_function_in_loop` | `unnecessary_host_function_call` |

### Existing lint names

The existing lint names:

- `soroban_storage_in_loop`
- `redundant_env_clone`
- `unnecessary_host_function_call`

are already part of the public interface. Although they do not fully match the convention above, they are retained as legacy names because renaming shipped lints would be a breaking change for users.

### 4. Code Quality Standards

All PRs are checked by CI, and these checks must pass before a PR can be merged. Run them locally before pushing:

1. Format your code with rustfmt (CI rejects unformatted code):

   ```bash
   cargo fmt --all
   ```

2. Make sure Clippy passes with no warnings:

   ```bash
   cargo clippy --workspace --all-targets -- -D warnings
   ```

3. Make sure the test suite passes:

   ```bash
   cargo test --workspace
   ```

Follow the patterns already used in the codebase: `soroban_cost_lints` uses edition 2024, so prefer let-chains (`if let ... && let ...`) over nested `if let` blocks, and match the structure of the existing lint passes when adding a new lint.

### 5. Upgrading the Nightly Toolchain

The pinned nightly is declared once in `rust-toolchain` (the single source of truth) and must stay in sync across multiple files. Upgrading is a multi-step, order-dependent process. The complete procedure—including identification of the matching `clippy_utils` revision, common breakages, and how to verify success—is documented in the [Nightly Upgrade Runbook](./docs/NIGHTLY_UPGRADE_RUNBOOK.md).

**TL;DR:** The critical relationship is between the nightly channel in `rust-toolchain` and the `clippy_utils` git revision in `soroban_cost_lints/Cargo.toml`. See the runbook for guidance on finding the matching revision from the `rust-lang/rust-clippy` repository's `rustup` branch.

### 6. Releasing a New Lint

A new lint is released as part of the next project version after its implementation has been merged. The release process is documented in [Releasing a New Lint](docs/release_process.md).

Before a release, maintainers must confirm that the lint is complete, documented, registered, covered by UI tests, and included in the changelog. The lint name must remain stable after release because it is part of the public configuration and command-line interface.

### 7. Submitting a Pull Request
- Ensure your PR targets the `main` branch.
- Make sure the checks in the section above (`cargo fmt`, `cargo clippy`, `cargo test`) all pass.
- Provide a clear description of what the lint does and why it saves costs.
- If your pull request includes a user-visible change, add an appropriate entry under the **Unreleased** section of `CHANGELOG.md`. The entry will be moved into the next versioned release when a release is cut.
