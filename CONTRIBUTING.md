# Contributing to `soroban-cost-linter`

First off, thank you for considering contributing to `soroban-cost-linter`!

## How to Contribute

### 1. Understanding the Architecture

This tool leverages Dylint to hook into the Rust compiler's AST and High-Level Intermediate Representation (HIR). Familiarity with `rustc` internals (like `rustc_hir`, `rustc_middle`) and `clippy` source code is highly beneficial.

### 2. Setting up Locally

1. Install Dylint:

```bash
   cargo install cargo-dylint dylint-link
```

2. Clone the repository and build:

```bash
   cargo build
```

3. Run tests:

```bash
   cargo test
```

### 3. Adding a New Lint

- Find a structural anti-pattern in Soroban that is input-independent and costly.
- Write a failing test case in the `ui` tests directory.
- Implement the lint using the `dylint` framework, checking the AST or HIR for the specific pattern.
- Update the documentation and `README.md`.

#### UI tests and updating `main.stderr`

Lints are exercised by a single UI test, `#[test] fn ui()` at the bottom of
[`soroban_cost_lints/src/lib.rs`](soroban_cost_lints/src/lib.rs):

```rust
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
```

This compiles every case in [`soroban_cost_lints/ui/main.rs`](soroban_cost_lints/ui/main.rs) with the lints enabled and diffs the compiler's actual diagnostics against the checked-in expected output in `soroban_cost_lints/ui/main.stderr`. **Both files are checked in and must be kept in sync** — if you add a case, or change a lint's message, help text, or span, `main.stderr` needs to be regenerated.

Run just this test with:

```bash
cargo test -p soroban_cost_lints ui
```

When the expected and actual output differ, the test fails with a `diff of stderr` report. **Do not hand-edit `main.stderr` to make the diff disappear** — editing it by hand tends to produce output that merely silences the test without actually matching what the compiler emits. Instead, re-bless it:

1. Run `cargo test -p soroban_cost_lints ui` and let it fail.
2. In the output, find the line `Actual stderr saved to <path>`. Note the path — the pinned test harness (`dylint_testing` 6.0.1, wrapping `compiletest_rs` 0.11.2) does **not** support a `BLESS=1`-style environment variable in this version (that only exists in newer releases of these crates), so the actual diagnostics are written to a temp-directory path rather than back into the repo. Check the versions in `soroban_cost_lints/Cargo.toml` / `Cargo.lock` before relying on `BLESS=1` from a blog post or another project — it is a no-op here.
3. Copy that file over the checked-in one: `cp <path> soroban_cost_lints/ui/main.stderr`.
4. **Review the resulting diff before committing it.** A regenerated file isn't automatically correct — read it to confirm the new diagnostic text/span is what you intended, and not, for example, a span shift caused by an unrelated edit elsewhere in `main.rs`.
5. Re-run `cargo test -p soroban_cost_lints ui` to confirm it now passes.

### Lint Naming Convention

Lint names are part of the project's public API. They appear in `#[allow(...)]`, `#[warn(...)]`, `#[deny(...)]`, and `budget.toml`, so renaming a shipped lint is a breaking change and should be avoided.

When adding new lints:

- Use lowercase `snake_case`.
- Prefer names that describe the code pattern being detected rather than expressing a judgement about the code.
- Use consistent suffixes such as `_in_loop` for loop-specific patterns.
- Avoid a `soroban_` prefix unless it is needed to avoid ambiguity with another well-known lint name.

Examples:

| Preferred | Avoid |
| ---------- | ----- |
| `storage_in_loop` | `soroban_storage_in_loop` |
| `env_clone` | `redundant_env_clone` |
| `host_function_in_loop` | `unnecessary_host_function_call` |

### Existing lint names

The existing lint names:

- `soroban_storage_in_loop`
- `redundant_env_clone`
- `unnecessary_host_function_call`

are already part of the public interface. Although they do not fully match the convention above, they are retained as legacy names because renaming shipped lints would be a breaking change for users.

### 4. Code Quality Standards

All PRs are checked by CI, and these checks must pass before a PR can be merged. Run them locally before pushing:

1. Format your code with rustfmt (CI rejects unformatted code):

```bash
   cargo fmt --all
```

2. Make sure Clippy passes with no warnings:

```bash
   cargo clippy --workspace --all-targets -- -D warnings
```

3. Make sure the test suite passes:

```bash
   cargo test --workspace
```

Follow the patterns already used in the codebase: `soroban_cost_lints` uses edition 2024, so prefer let-chains (`if let ... && let ...`) over nested `if let` blocks, and match the structure of the existing lint passes when adding a new lint.

### 5. Submitting a Pull Request

- Ensure your PR targets the `main` branch.
- Make sure the checks in the section above (`cargo fmt`, `cargo clippy`, `cargo test`) all pass.
- Provide a clear description of what the lint does and why it saves costs.
- If your pull request includes a user-visible change, add an appropriate entry under the **Unreleased** section of `CHANGELOG.md`. The entry will be moved into the next versioned release when a release is cut.
