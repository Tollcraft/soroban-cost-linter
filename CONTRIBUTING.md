# Contributing to `soroban-cost-linter`

First off, thank you for considering contributing to `soroban-cost-linter`!

## How to Contribute

### 1. Understanding the Architecture

This tool leverages Dylint to hook into the Rust compiler's AST and High-Level Intermediate Representation (HIR). Familiarity with `rustc` internals (like `rustc_hir`, `rustc_middle`) and `clippy` source code is highly beneficial.

### 2. Getting Started

#### Recommended: Dev Container

The project includes a pre-configured dev container image with the exact nightly toolchain,
compiler components (`rustc-dev`, `llvm-tools-preview`), and Dylint binaries already installed.

**VS Code / GitHub Codespaces:** open this repo and choose "Reopen in Container" when prompted.
The `.devcontainer/devcontainer.json` will build and launch the container automatically.

**Standalone Docker:**

```bash
docker build -t soroban-cost-linter .
docker run --rm -it -v "$(pwd)":/workspace soroban-cost-linter bash
# Inside the container:
cargo test --workspace
```

A prebuilt image is also published to the GitHub Container Registry on every push to `main`
that touches the Dockerfile or the toolchain pin:

```bash
docker pull ghcr.io/Tollcraft/soroban-cost-linter:latest
```

#### Manual Local Setup

If you prefer to set up the toolchain on your machine directly:

1. Install Dylint:

   ```bash
   cargo install cargo-dylint dylint-link --version "^6.0.1"
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
- Read the [Scope: Clippy vs. soroban-cost-linter](../docs/scope_boundary.md) guide first. If a pattern is already covered by a Clippy lint and the Soroban cost story does not change the analysis, do not duplicate it here.
- Find a structural anti-pattern in Soroban that is input-independent and costly, and that is **not** already covered by a Clippy lint with the same cost-relevant semantics.
- Assign the lint to one of the defined cost categories (`StorageOperations`, `Compute`, `Memory`, or `EntryLifecycle`) and add it to the `LINT_METADATA` registry in `soroban_cost_lints/src/lib.rs`.
- Write a failing test case in the `ui` tests directory.
- Implement the lint using the `dylint` framework, checking the AST or HIR for the specific pattern.
- Update the documentation and `README.md`, ensuring the new lint is placed under the correct category header.

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

### 5. Upgrading the Nightly Toolchain

The pinned nightly is declared once in `rust-toolchain` (the single source of truth) and must stay in sync across four files, the `clippy_utils` git rev in `soroban_cost_lints/Cargo.toml`, and the container image.

**Procedure to upgrade:**

1. Update `rust-toolchain` with the new nightly date (e.g. `nightly-2026-05-01`).
2. Find the matching `clippy_utils` commit from the [`rust-lang/rust-clippy`](https://github.com/rust-lang/rust-clippy) repository's `rustup` branch on that date, and update the `rev` field in `soroban_cost_lints/Cargo.toml`.
3. Update `.github/workflows/lint.yml`, `templates/github-action.yml`, and `docs/integration.md` with the new nightly date.
4. Run the drift guard to confirm everything agrees:
   ```bash
   bash .github/scripts/validate-toolchain-pins.sh
   ```
5. Run the full test suite:
   ```bash
   cargo test --workspace
   ```

If any file is out of sync, the drift guard will print an error naming the file, the mismatched value, and the expected one.

{% hint style="info" %}
The `Dockerfile` reads `rust-toolchain` at build time, so updating the channel there is sufficient — the container image will be rebuilt and published automatically by CI when `rust-toolchain` changes.
{% endhint %}

### 6. Submitting a Pull Request
- Ensure your PR targets the `main` branch.
- Make sure the checks in the section above (`cargo fmt`, `cargo clippy`, `cargo test`) all pass.
- Provide a clear description of what the lint does and why it saves costs.
- If your pull request includes a user-visible change, add an appropriate entry under the **Unreleased** section of `CHANGELOG.md`. The entry will be moved into the next versioned release when a release is cut.
