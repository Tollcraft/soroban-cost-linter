# Integration Guide

`soroban-cost-linter` integrates directly into your workspace and CI/CD pipelines.

## Colour Control

`soroban-cost-linter` forwards rustc's own coloured diagnostics to the terminal.
You can control this behaviour with the `--color` flag or the `NO_COLOR`
environment variable.

| Scenario | Result |
|---|---|
| `cargo cost-lint` (no flags) | Colours when stdout is a terminal, none otherwise |
| `cargo cost-lint --color always` | Colours always, even when piped |
| `cargo cost-lint --color never` | No colours ever |
| `NO_COLOR=1 cargo cost-lint` | No colours (same as `--color never`) |
| `NO_COLOR=1 cargo cost-lint --color always` | Colours (`--color` takes precedence) |

The [NO_COLOR](https://no-color.org/) convention is respected: any non-empty
value forces uncoloured output unless `--color` is passed explicitly.

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
| `string_concat_in_loop`            | `warn`        |
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

`cargo-cost-lint` supports generating shell completion scripts. Use the `--completions` flag to generate a script for your shell.

### Bash
```bash
cargo cost-lint --completions bash > ~/.local/share/bash-completion/completions/cargo-cost-lint
```

### Zsh
```zsh
cargo cost-lint --completions zsh > _cargo-cost-lint
# Ensure it's in your fpath
```

### Fish
```fish
cargo cost-lint --completions fish > ~/.config/fish/completions/cargo-cost-lint.fish
```

## Colour Control

... (rest of the file remains same) ...