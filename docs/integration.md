# Integration Guide

`soroban-cost-linter` integrates directly into your workspace, CI/CD pipelines, and editor.

## Prerequisites

- Rust nightly toolchain (pinned in `rust-toolchain`)
- [Dylint](https://github.com/trailofbits/dylint) `^6.0.1`
- `cargo-cost-lint` installed

```bash
# Install Dylint
cargo install cargo-dylint dylint-link --version "^6.0.1"

# Install the linter
cargo install --git https://github.com/Tollcraft/soroban-cost-linter.git cargo-cost-lint
```

## Quick Start

```bash
# Run the linter on your Soroban project
cargo cost-lint
```

## Local Configuration (`budget.toml`)

Create a `budget.toml` file in the root of your cargo workspace to adjust lint severities:

The tool locates `budget.toml` by walking up from the current directory until it finds a `Cargo.toml` containing a `[workspace]` section, then looks for `budget.toml` in that directory. This means running `cargo cost-lint` from any member crate produces the same lint levels as running it from the workspace root.

You can also pass an explicit path with `--config <PATH>`, which is used verbatim relative to the current directory.

{% code title="budget.toml" %}
```toml
[lints]
soroban_storage_in_loop = "deny"       # default: deny (high confidence)
redundant_env_clone = "warn"           # default: warn
unnecessary_host_function_call = "warn" # default: warn
bytes_append_in_loop = "warn"          # default: warn
symbol_new_for_short_literal = "warn"  # default: warn
host_in_loop = "warn"                  # default: warn
```
{% endcode %}

{% hint style="warning" %}
**Breaking change:** `soroban_storage_in_loop` was upgraded from `warn` to `deny` in v0.2.0. If this breaks your CI, set `soroban_storage_in_loop = "warn"` in `budget.toml` to restore the previous behaviour.
{% endhint %}

{% hint style="info" %}
See the [Lint Reference](lints/) for what each lint catches and its default severity.
{% endhint %}

### Validation

`cargo cost-lint` strictly validates your `budget.toml`:
- If an unknown lint **name** is provided (e.g., due to a typo), the tool will print an error listing valid lints and exit immediately. This ensures a mistyped `deny` cannot silently fail to apply.
- If an unknown lint **level** is provided, the tool will emit an error and exit immediately. Valid levels are `allow`, `warn`, and `deny`.

## Editor Integration

### VS Code with rust-analyzer

The linter provides inline diagnostics in VS Code by configuring rust-analyzer to use `cargo cost-lint` as its check command.

#### `settings.json`

Add the following to your VS Code workspace or user settings:

```json
{
  "rust-analyzer.check.overrideCommand": [
    "cargo",
    "cost-lint",
    "--all-diagnostics"
  ]
}
```

#### How it works

- This replaces rust-analyzer's normal `cargo check` invocation with `cargo cost-lint --all-diagnostics`.
- The `--all-diagnostics` flag ensures both regular compiler diagnostics AND soroban cost-lint diagnostics appear inline.
- Diagnostics use the standard rustc format, so errors show as red squiggles and warnings as yellow squiggles.
- Deny-level lints (e.g. `soroban_storage_in_loop`) appear as errors.
- The command is run in the background whenever you modify a file.

#### Example

After configuration, a storage operation inside a loop will show an inline error:

```rust
for item in items {
    env.storage().instance().set(&item, &1);
    // ^^^ error: storage operation inside a loop
}
```

#### Requirements

- `cargo-dylint` and `dylint-link` must be installed and on `$PATH`.
- `cargo-cost-lint` must be installed from the same Soroban nightly toolchain used in your project.
- The first invocation builds the lint library, which may take a minute. Subsequent runs are cached.

#### Limitations

- `cargo cost-lint` replaces `cargo check` entirely. You will still see all regular compile errors and warnings, but the invocation path is different.
- `budget.toml` lint-level overrides are applied by the CLI wrapper and work the same as in terminal mode.
- The lint library must be compiled for the exact nightly toolchain your project uses. A mismatch produces a link error.

#### Troubleshooting

| Symptom | Likely Cause | Fix |
|---|---|---|
| No diagnostics appear | `cargo-dylint` or `dylint-link` not installed | Run `cargo install cargo-dylint dylint-link --version "^6.0.1"` |
| Lint diagnostics appear but regular errors don't | `--all-diagnostics` flag is missing | Add `"--all-diagnostics"` to `overrideCommand` |
| `error: no such command: dylint` | `cargo-dylint` not in `$PATH` | Install dylint and restart rust-analyzer |
| Link errors about mismatched toolchain | Nightly toolchain mismatch | Ensure `rust-toolchain` matches the installed `cargo-cost-lint` |
| Diagnostics are stale | rust-analyzer cache | Run "Rust Analyzer: Restart Server" from the command palette |

#### Disabling the Integration

To revert to normal `cargo check` behaviour, remove the `overrideCommand` setting from your VS Code configuration:

```json
{
  "rust-analyzer.check.overrideCommand": null
}
```

### Other Editors

Any editor with rust-analyzer LSP support can use the same configuration:

- **Neovim** (rustaceanvim): Set `rust-analyzer.check.overrideCommand` in your LSP settings.
- **Helix**: Set `check.overrideCommand` in `.helix/languages.toml`.
- **Emacs** (eglot): Pass the same settings via `lsp-register-client` or `eglot-workspace-configuration`.

The principle is the same: point the check command at `cargo cost-lint --all-diagnostics`.

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
