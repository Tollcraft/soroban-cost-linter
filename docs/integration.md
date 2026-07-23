# Integration Guide

`soroban-cost-linter` integrates directly into your workspace and CI/CD pipelines.

## Local Configuration (`budget.toml`)

Create a `budget.toml` file in the root of your cargo workspace to adjust lint severities:

{% code title="budget.toml" %}
```toml
[lints]
soroban_storage_in_loop = "deny"
redundant_env_clone = "warn"
unnecessary_host_function_call = "warn"
```
{% endcode %}

{% hint style="info" %}
See the [Lint Reference](lints/) for what each lint catches and its default severity.
{% endhint %}

{% hint style="danger" %}
A `budget.toml` that exists but cannot be read or parsed is a hard error — the run will fail with a non-zero exit code and the parser's message and location will be reported.
{% endhint %}

## GitHub Actions

We provide a template to easily integrate the linter into your GitHub Actions pipeline:

{% code title=".github/workflows/cost-lint.yml" %}
```yaml
name: Soroban Cost Lint

on: [push, pull_request]

jobs:
  cost-lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          # Keep this toolchain pin in sync with the soroban-cost-linter release you install
          toolchain: nightly-2026-04-16
          components: rustc-dev, llvm-tools-preview
      - name: Install Dylint
        run: cargo install cargo-dylint dylint-link
      - name: Install soroban-cost-linter
        run: cargo install --git https://github.com/Tollcraft/soroban-cost-linter.git cargo-cost-lint
      - name: Run Cost Linter
        run: cargo cost-lint
```
{% endcode %}

{% hint style="warning" %}
Keep the pinned `toolchain` in sync with the `soroban-cost-linter` release you install — a mismatched nightly will fail to link the lint library.
{% endhint %}
