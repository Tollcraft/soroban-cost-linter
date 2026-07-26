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
