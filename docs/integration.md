# Integration Guide

`soroban-cost-linter` integrates directly into your workspace and CI/CD pipelines.

## Local Configuration (`budget.toml`)

Create a `budget.toml` file to adjust lint severities across your workspace.

Supply it via the `--config <PATH>` flag:

```bash
cargo cost-lint --config budget.toml
```

If the path does not exist or is not a valid TOML file, it is silently ignored.

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

### Lint levels

Each value must be one of the three standard Rust lint levels:

| Level   | Behaviour                                              |
|---------|--------------------------------------------------------|
| `allow` | Suppress the lint entirely                             |
| `warn`  | Produce a warning (default for all lints)              |
| `deny`  | Produce a hard error — fails the build                 |

A level that is not one of these three currently produces **no error**; the entry is silently skipped. (This will become a hard validation in a future release — see [#177](https://github.com/Tollcraft/soroban-cost-linter/issues/177).)

### Lint names

Each key under `[lints]` must match a lint name **exactly** as shown in the compiler output. The known names are:

| Lint name                           | Default level |
|-------------------------------------|---------------|
| `soroban_storage_in_loop`           | `warn`        |
| `redundant_env_clone`               | `warn`        |
| `unnecessary_host_function_call`    | `warn`        |
| `symbol_new_for_short_literal`      | `warn`        |

A key that does not match one of these names currently produces **no error**; the entry is silently skipped. (Exhaustive validation against this list is tracked in [#177](https://github.com/Tollcraft/soroban-cost-linter/issues/177).)

### How it reaches the compiler

`cargo cost-lint` applies budget.toml levels by building a `DYLINT_RUSTFLAGS` string that it passes to `cargo dylint`. Dylint forwards these flags to `rustc` as `-A`/`-W`/`-D` directives.

If `DYLINT_RUSTFLAGS` is already set in your shell environment, the tool **appends** to it instead of replacing it:

```
User env:    DYLINT_RUSTFLAGS=-Wsome_other_lint
Tool adds:                      -A<soroban_storage_in_loop>
Result:      DYLINT_RUSTFLAGS=-Wsome_other_lint -A<soroban_storage_in_loop>
```

### Precedence

Rustc resolves the effective lint level in this order (highest priority first):

1. `--force-warn` / `--cap-lints` (never set by this tool)
2. `#[allow]` / `#[warn]` / `#[deny]` attributes in source code
3. Compiler flags from `DYLINT_RUSTFLAGS` (the mechanism used by budget.toml)
4. The lint's built-in default

A `deny` in `budget.toml` raises the level from `warn` (the default) to `deny`. An `#[allow]` attribute on a function body suppresses a `warn`-level lint for that function just as it normally would — but a `deny` in `budget.toml` will cause that same function to fail, because `#[allow]` cannot override `-D`. Conversely, `allow` in budget.toml suppresses the lint everywhere, even overriding `#[deny]` in source code.

### Current limitations

- **The `[lints]` section is parsed but its contents are currently discarded.** The documentation above describes the *intended* design. Until the validation and flag-building logic is wired up (tracked in [#177](https://github.com/Tollcraft/soroban-cost-linter/issues/177)), budget.toml has no effect on the lint output. The file exists now so that users can set up the right file structure ahead of time; `cargo cost-lint --config budget.toml` will succeed but produce the same results as without it.

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
