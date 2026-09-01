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

## GitHub Actions

The easiest way to add Soroban cost linting to your CI is the official composite
action, `Tollcraft/soroban-cost-linter`. It installs the pinned Rust toolchain
and Dylint, builds the lint library and the `cargo cost-lint` CLI, and runs the
linter against your contract, emitting GitHub workflow annotations for every
finding. You do **not** need to hand-roll the toolchain install.

Copy this to `.github/workflows/cost-lint.yml` in your contract workspace:

```yaml
name: Soroban Cost Lint

on: [push, pull_request]

jobs:
  cost-lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run soroban-cost-linter
        uses: Tollcraft/soroban-cost-linter@v1
        with:
          config: budget.toml
```

The workflow checks out your repository, then invokes the action. The `with:`
block wires the `config` input to your `budget.toml` (the same file described
in [Local Configuration](#local-configuration-budgettoml)). If you have more
than one contract workspace in a monorepo, point the action at the right one
with `working-directory: <path>`, and pass any extra CLI flags through `args:`.

The `toolchain` input defaults to the nightly that matches the release
(`nightly-2026-04-16`). Leave it unset unless you specifically need a different
nightly — a mismatched toolchain is what makes the lint library fail to link.

### Input reference

The action accepts four inputs:

| Input | Description | Default |
|---|---|---|
| `config` | Path to `budget.toml`, relative to `working-directory` | *(none — every lint runs at its default level)* |
| `toolchain` | Rust nightly toolchain version | `nightly-2026-04-16` |
| `args` | Additional arguments to pass to `cargo cost-lint` | *(none)* |
| `working-directory` | Directory containing the contract workspace to lint | `.` |

For example, to lint a subdirectory with an extra `--format` override:

```yaml
- name: Run soroban-cost-linter
  uses: Tollcraft/soroban-cost-linter@v1
  with:
    working-directory: contracts/my-account
    config: ../shared/budget.toml
    args: --format text
```

### What a failing run looks like

Most shipped lints default to `warn`, but `soroban_storage_in_loop` defaults to
`deny`. Because `cargo cost-lint` exits `1` whenever any finding is
`deny`/`error` level, a storage operation inside a loop fails the job by
default — no config needed. To also fail on the other lints, raise them to
`deny` in your `budget.toml` (wired in via the `config` input):

```toml
[lints]
redundant_env_clone = "deny"
```

A run that produces only `warn` findings annotates the diff but still exits `0`
and reports green. Every finding is printed as a GitHub annotation of the form
`::error file=src/lib.rs,line=12,col=5::storage operation inside a loop`
(shown inline in the pull-request diff). A failing run therefore shows a
`deny`-level annotation in the diff **and** fails the job with exit code `1` —
that is how you tell a real finding from a broken setup, which would fail while
building the toolchain or the lint library instead.

{% hint style="warning" %}
The action's CI is exercised on `ubuntu-latest` only. If you target Windows,
test it in your own CI before relying on it — the steps install `cargo-dylint`
from source, which is the step most likely to differ from Linux.
{% endhint %}

## Local Configuration (`budget.toml`)

Create a `budget.toml` file to adjust lint severities, then point `cargo cost-lint` at it with `--config`. Today the only way to apply a config is to pass `--config <PATH>` explicitly — the tool does **not** automatically walk up to a workspace-root `budget.toml`. When `--config` is omitted, every lint runs at its declared default level (`warn` for most lints, `deny` for `soroban_storage_in_loop`).

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
