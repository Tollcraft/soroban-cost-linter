# Contributing to `soroban-cost-linter`

First off, thank you for considering contributing to `soroban-cost-linter`!

## How to Contribute

### 1. Understanding the Architecture
This tool leverages Dylint to hook into the Rust compiler's AST and High-Level Intermediate Representation (HIR). Familiarity with `rustc` internals (like `rustc_hir`, `rustc_middle`) and `clippy` source code is highly beneficial.

### 2. Setting up Locally
1. Install Dylint:
   ```bash
   cargo install cargo-dylint dylint-link
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
- Find a structural anti-pattern in Soroban that is input-independent and costly.
- Write a failing test case in the `ui` tests directory.
- Implement the lint using the `dylint` framework, checking the AST or HIR for the specific pattern.
- Update the documentation and `README.md`.

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

The pinned nightly is declared once in `rust-toolchain` (the single source of truth) and must stay in sync across four files and the `clippy_utils` git rev in `soroban_cost_lints/Cargo.toml`.

**Procedure to upgrade:**

1. Update `rust-toolchain` with the new nightly date (e.g. `nightly-2026-05-01`).
2. Find the matching `clippy_utils` commit from the [`rust-lang/rust-clippy`](https://github.com/rust-lang/rust-clippy) repository's `rustup` branch on that date, and update the `rev` field in `soroban_cost_lints/Cargo.toml`.
3. Update `.github/workflows/lint.yml`, `templates/github-action.yml`, and `docs/integration.md` with the new nightly date.
4. Run the drift guard to confirm everything agrees:
   ```bash
   bash .github/scripts/validate-toolchain-pins.sh
   ```
5. Run the full test suite:
   ```bash
   cargo test --workspace
   ```

If any file is out of sync, the drift guard will print an error naming the file, the mismatched value, and the expected one.

### 6. Submitting a Pull Request
- Ensure your PR targets the `main` branch.
- Make sure the checks in the section above (`cargo fmt`, `cargo clippy`, `cargo test`) all pass.
- Provide a clear description of what the lint does and why it saves costs.
