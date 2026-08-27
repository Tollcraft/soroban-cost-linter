# macOS Setup Guide

This page complements [`CONTRIBUTING.md`](../CONTRIBUTING.md) with macOS-specific setup instructions for `soroban-cost-linter`.

## Environment & Prerequisites

1. **Rust Toolchain**: Install via [`rustup`](https://rustup.rs):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
2. **Apple Command Line Tools / Xcode**:
   ```bash
   xcode-select --install
   ```

## Installing Dylint and the Linter

```bash
# Install Dylint (matching version pin)
cargo install cargo-dylint dylint-link --version "^6.0.1" --locked

# Install cargo-cost-lint wrapper
cargo install --path cargo-cost-lint
```

## Running the Linter on macOS

```bash
cd path/to/my-soroban-contract
cargo cost-lint
```

## CI Matrix & Cache Notes

- The project CI test matrix includes `macos-latest`.
- Dynamic library compilation (`cargo-dylint` and `dylint-link`) on macOS utilizes shared rust-cache keys (`cost-linter`) to maintain fast CI build times.
