# Integration Guide

`soroban-cost-linter` integrates directly into your workspace and CI/CD pipelines.## Local Configuration (`budget.toml`)

Create a `budget.toml` file in the root of your cargo workspace to adjust lint severities.  This file is **shared** with [`soroban-budget-assert`](https://github.com/Tollcraft/soroban-budget-assert) — both tools read their own sections from the same file, so you only need one `budget.toml` per project.

The tool locates `budget.toml` by walking up from the current directory until it finds a `Cargo.toml` containing a `[workspace]` section, then looks for `budget.toml` in that directory. This means running `cargo cost-lint` from any member crate produces the same lint levels as running it from the workspace root.

You can also pass an explicit path with `--config <PATH>`, which is used verbatim relative to the current directory.

### Full schema

{% code title="budget.toml" %}
```toml
# ── soroban-cost-linter section ───────────────────────────────────────
# Every key is a lint name; the value must be "allow", "warn", or "deny".
# Unknown lint names are rejected at parse time — a typo will not be
# silently ignored.

[lints]
soroban_storage_in_loop = "deny"
redundant_env_clone = "warn"
unnecessary_host_function_call = "warn"

# ── soroban-budget-assert sections ────────────────────────────────────
# These are consumed by the sibling runtime test harness.  They are
# preserved verbatim so both tools can coexist in one file.

[network]
rpc_url = "https://soroban-testnet.stellar.org"

[source]
account = "G..."

[functions.my_contract]
max_cpu_instructions = 100_000_000
```
{% endcode %}

### Section ownership

| Section | Owner | Behaviour |
|---------|-------|-----------|
| `[lints]` | `soroban-cost-linter` | Strictly validated; unknown keys error |
| `[network]` | `soroban-budget-assert` | Ignored by linter (foreign section) |
| `[source]` | `soroban-budget-assert` | Ignored by linter (foreign section) |
| `[functions.*]` | `soroban-budget-assert` | Ignored by linter (foreign section) |

{% hint style="info" %}
See the [Lint Reference](lints/) for what each lint catches and its default severity.
{% endhint %}

### Validation

`cargo cost-lint` strictly validates your `budget.toml`:
- If an unknown lint **name** is provided (e.g., due to a typo), the tool will print an error listing valid lints and exit immediately. This ensures a mistyped `deny` cannot silently fail to apply.
- If an unknown lint **level** is provided, the tool will emit an error and exit immediately. Valid levels are `allow`, `warn`, and `deny`.

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
        run: cargo install cargo-dylint dylint-link --version "^6.0.1"
      - name: Install soroban-cost-linter
        run: cargo install --git https://github.com/Tollcraft/soroban-cost-linter.git cargo-cost-lint
      - name: Run Cost Linter
        run: cargo cost-lint
```
{% endcode %}

{% hint style="warning" %}
Keep the pinned `toolchain` in sync with the `soroban-cost-linter` release you install — a mismatched nightly will fail to link the lint library.
{% endhint %}

## JSON Output and CI Annotations

For machine-readable output, pass `--format json`. `cargo cost-lint` will emit JSON lines (NDJSON) detailing each lint finding. The exit code remains non-zero if a `deny` level lint fires.

### JSON Schema
Each line of stdout is a JSON object with the following schema:
```json
{
  "name": "soroban_storage_in_loop",
  "level": "warning",
  "file": "src/lib.rs",
  "span": {
    "line_start": 42,
    "line_end": 42,
    "column_start": 13,
    "column_end": 18
  },
  "message": "storage operations in loops are expensive",
  "help": "consider lifting the storage operation outside the loop"
}
```

### GitHub Actions Annotations Example
You can pipe the JSON output into a tool like `jq` to create GitHub annotations (which show up directly on your PR's Files Changed tab).

```yaml
      - name: Run Cost Linter (JSON mode with annotations)
        run: |
          cargo cost-lint --format json | jq -r '
            . | "::\(.level) file=\(.file),line=\(.span.line_start),col=\(.span.column_start)::\(.message) (Lint: \(.name))"
          '
```
*(Note: If the linter returns a non-zero exit code due to a `deny` lint, the step will still fail correctly in Actions).*
