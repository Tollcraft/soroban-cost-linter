# Integration Guide

`soroban-cost-linter` integrates directly into your workspace and CI/CD pipelines.## Local Configuration (`budget.toml`)

## Local Configuration (`budget.toml`)

Create a `budget.toml` file to adjust lint severities, then point `cargo cost-lint` at it with `--config`. Today the only way to apply a config is to pass `--config <PATH>` explicitly — the tool does **not** automatically walk up to a workspace-root `budget.toml`. When `--config` is omitted, every lint runs at its declared default level (currently `warn` for all shipped lints).

The `--config` flag accepts a single path (relative or absolute). A relative path is resolved against the directory you run `cargo cost-lint` from; an absolute path is used verbatim.

**Example — config in a subdirectory:**

```bash
cargo cost-lint --config ./configs/strict.budget.toml
```

**Example — config at an absolute path:**

```bash
cargo cost-lint --config /etc/soroban-cost-linter/budget.toml
```

`budget.toml` may live anywhere on disk; this flag is the single supported way to point the tool at it. The path you pass goes through the same `BudgetConfig` parser regardless of location, so unknown lint names or invalid levels fail validation identically.

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

### Lint levels

Each value must be one of the three standard Rust lint levels:

| Level   | Behaviour                                              |
|---------|--------------------------------------------------------|
| `allow` | Suppress the lint entirely                             |
| `warn`  | Produce a warning (default for all lints)              |
| `deny`  | Produce a hard error — fails the build                 |

A level that is not one of `allow`, `warn`, or `deny` causes the tool to print an error and exit immediately.

### Lint names

Each key under `[lints]` must match a lint name **exactly** as shown in the compiler output. The known names are:

| Lint name                           | Default level |
|-------------------------------------|---------------|
| `soroban_storage_in_loop`           | `warn`        |
| `redundant_env_clone`               | `warn`        |
| `unnecessary_host_function_call`    | `warn`        |
| `symbol_new_for_short_literal`      | `warn`        |
| `bytes_append_in_loop`              | `warn`        |
| `storage_write_without_read`        | `warn`        |
| `inefficient_bytes_concat`          | `warn`        |
| `map_insert_in_loop`                | `warn`        |
| `host_in_loop`                      | `warn`        |

An unknown lint name causes the tool to print an error listing valid lints and exit immediately.

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

## Colour Control

`soroban-cost-lint` does not colour anything itself, but in text mode it spawns `cargo dylint` with inherited stdio, so rustc's coloured diagnostics come straight through. The wrapper provides a `--color` flag and honours the `NO_COLOR` convention to control this.

| Flag | Behaviour |
|------|----------|
| `--color auto` (default) | Colour when stdout is a terminal, none when redirected or piped |
| `--color always` | Always emit colour, even when piped |
| `--color never` | Never emit colour |

The `NO_COLOR` environment variable (set to any non-empty value, per [no-color.org](https://no-color.org/)) disables colour when `--color` is not explicitly passed. The explicit `--color` flag always takes precedence.

**Examples:**

```bash
# Default — colour in terminal, none when piped
cargo cost-lint | less -R

# Force colour off (useful for CI logs)
NO_COLOR=1 cargo cost-lint

# Force colour on (useful for debugging)
cargo cost-lint --color always
```

In JSON mode (`--format json`), colour is never emitted — stdout is already piped and contains only NDJSON.

## Editor / IDE Integration

`soroban-cost-linter` can surface lint warnings directly in your editor through **rust-analyzer**'s check override mechanism. This works in any editor that supports rust-analyzer (VS Code, Zed, Helix, Neovim, etc.).

{% hint style="info" %}
**Prerequisites:** You must have `cargo-dylint`, `dylint-link`, and `cargo-cost-lint` [installed](../README.md#installation) before configuring IDE integration.
{% endhint %}

### How it works

rust-analyzer runs `cargo check` by default to provide real-time diagnostics. By overriding the check command to use `cargo dylint` with the `soroban_cost_lints` library, the linter's output is parsed and displayed as standard warnings and errors right in your editor's problem panel. This mirrors the same `cargo dylint` invocation that `cargo cost-lint` uses internally.

### VS Code setup

Add the following to your workspace's `.vscode/settings.json`:

```json
{
    "rust-analyzer.check.overrideCommand": [
        "cargo",
        "dylint",
        "--lib",
        "soroban_cost_lints",
        "--",
        "--all-targets",
        "--message-format=json"
    ]
}
```

Once saved, rust-analyzer will restart its check process. Lint findings will appear in the **Problems** panel (Ctrl+Shift+M) with the same formatting shown in the [Usage](../README.md#usage) section.

{% hint style="warning" %}
Dylint-based IDE integration relies on `rust-analyzer.check.overrideCommand`, which replaces the default `cargo check` entirely. This is a stable rust-analyzer feature and is the approach [recommended by Dylint](https://github.com/trailofbits/dylint), but it is not tested against every editor and Rust toolchain combination. If you encounter issues, please [file a bug report](https://github.com/Tollcraft/soroban-cost-linter/issues/new?template=bug_report.yml).
{% endhint %}

### Other editors

Any editor that uses rust-analyzer can apply the same override. Consult your editor's rust-analyzer configuration documentation for equivalent settings:

- **Zed:** `"lsp": { "rust-analyzer": { "check": { "overrideCommand": [...] } } }` in your project settings
- **Helix:** `[language-server.rust-analyzer.config.check]` in `languages.toml`
- **Neovim (lspconfig):** `settings = { ["rust-analyzer"] = { check = { overrideCommand = {...} } } }`

### Performance considerations

{% hint style="warning" %}
Running `cargo dylint` on every save is **slower** than the default `cargo check`, because it loads and executes dynamic lint libraries in addition to the compiler's normal analysis pass. For most Soroban projects the overhead is modest, but it scales with project size.
{% endhint %}

If the performance overhead is too high for daily development, consider these alternatives:

- **On-demand only:** Remove the override from your workspace settings and run `cargo cost-lint` manually in a terminal when you want lint feedback.
- **CI-only:** Keep the linter in your [GitHub Actions](#github-actions) pipeline and rely on PR checks for enforcement.

## GitHub Actions

We provide a template to easily integrate the linter into your GitHub Actions pipeline. The template runs on both Linux and Windows:

{% code title=".github/workflows/cost-lint.yml" %}
```yaml
name: Soroban Cost Lint

on: [push, pull_request]

jobs:
  cost-lint:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4
      - name: Install Rust
        uses: dtolnay/rust-toolchain@e97e2d8cc328f1b50210efc529dca0028893a2d9 # v1
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
