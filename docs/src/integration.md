# Integration Guide

`soroban-cost-linter` integrates directly into your workspace and CI/CD pipelines.

## Local Configuration (`budget.toml`)
Create a `budget.toml` file in the root of your cargo workspace to adjust lint severities:

```toml
[lints]
soroban_storage_in_loop = "deny"
redundant_env_clone = "warn"
unnecessary_host_function_call = "warn"
```

## GitHub Actions
We provide a template to easily integrate the linter into your GitHub Actions pipeline:

```yaml
name: Soroban Cost Lint

on: [push, pull_request]

jobs:
  cost-lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: nightly
      - name: Install Dylint
        run: cargo install cargo-dylint dylint-link
      - name: Install soroban-cost-linter
        run: cargo install --git https://github.com/Tollcraft/soroban-cost-linter.git cargo-cost-lint
      - name: Run Cost Linter
        run: cargo cost-lint
```
