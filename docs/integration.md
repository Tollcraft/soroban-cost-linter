# Integration Guide

`soroban-cost-linter` integrates directly into your workspace and CI/CD pipelines.

## Local Configuration (`budget.toml`)

Create a `budget.toml` file in the root of your cargo workspace to adjust lint severities:

The tool locates `budget.toml` by walking up from the current directory until it finds a `Cargo.toml` containing a `[workspace]` section, then looks for `budget.toml` in that directory. This means running `cargo cost-lint` from any member crate produces the same lint levels as running it from the workspace root.

You can also pass an explicit path with `--config <PATH>`, which is used verbatim relative to the current directory.

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

### Validation

`cargo cost-lint` strictly validates your `budget.toml`:
- If an unknown lint **name** is provided (e.g., due to a typo), the tool will print an error listing valid lints and exit immediately. This ensures a mistyped `deny` cannot silently fail to apply.
- If an unknown lint **level** is provided, the tool will emit an error and exit immediately. Valid levels are `allow`, `warn`, and `deny`.

## GitHub Actions

The recommended way to integrate the linter into CI is the reusable composite action:

{% code title=".github/workflows/cost-lint.yml" %}
```yaml
name: Soroban Cost Lint

on: [push, pull_request]

jobs:
  cost-lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: Tollcraft/soroban-cost-linter@v1
```
{% endcode %}

That single `uses:` line handles installing the correct Rust nightly, `cargo-dylint`, `dylint-link`, and the linter itself — then runs `cargo cost-lint`. A `deny`-level finding fails the job.

### With a custom budget.toml

```yaml
      - uses: Tollcraft/soroban-cost-linter@v1
        with:
          config: path/to/budget.toml
```

### In a monorepo subdirectory

```yaml
      - uses: Tollcraft/soroban-cost-linter@v1
        with:
          working-directory: contracts/my-soroban-project
```

### Passing extra arguments

```yaml
      - uses: Tollcraft/soroban-cost-linter@v1
        with:
          args: '--format json'
```

### Pinning a specific toolchain

```yaml
      - uses: Tollcraft/soroban-cost-linter@v1
        with:
          toolchain: nightly-2026-04-16
```

{% hint style="info" %}
The action defaults to the toolchain that matches the release. Override it only if you need a specific nightly for compatibility with your workspace.
{% endhint %}

### Full input reference

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `config` | No | `''` | Path to `budget.toml` (relative to `working-directory`) |
| `toolchain` | No | `nightly-2026-04-16` | Rust nightly toolchain version |
| `args` | No | `''` | Extra arguments forwarded to `cargo cost-lint` |
| `working-directory` | No | `'.'` | Directory containing the Soroban workspace |

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
      - uses: Tollcraft/soroban-cost-linter@v1
        with:
          args: '--format json'
      - name: Create GitHub annotations
        if: always()
        run: |
          # If you captured the JSON output to a file, parse it:
          # cargo cost-lint --format json > lint-results.json
          # jq -r '. | "::\(.level) file=\(.file),line=\(.span.line_start),col=\(.span.column_start)::\(.message) (Lint: \(.name))"' lint-results.json
```
*(Note: If the linter returns a non-zero exit code due to a `deny` lint, the step will still fail correctly in Actions).*
