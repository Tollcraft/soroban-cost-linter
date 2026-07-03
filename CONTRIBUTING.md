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

### 4. Submitting a Pull Request
- Ensure your PR targets the `v1` branch.
- Make sure `cargo test` passes.
- Provide a clear description of what the lint does and why it saves costs.
