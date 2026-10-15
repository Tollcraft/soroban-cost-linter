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

> **Windows users:** see [`docs/windows_setup.md`](docs/windows_setup.md) for
> WSL2 and native-PowerShell instructions. The commands below work on Linux and
> macOS, and on Windows inside WSL2 with Ubuntu.

#### Linux / macOS

1. Install Dylint:

   ```bash
   cargo install cargo-dylint dylint-link --version "^6.0.1" --locked
   ```

2. Clone the repository and build:

   ```bash
   git clone https://github.com/Tollcraft/soroban-cost-linter.git
   cd soroban-cost-linter
   cargo build
   ```

3. Run tests:

   ```bash
   cargo test --workspace
   ```

#### Windows (PowerShell)

1. **Install the Rust toolchain.** Use [rustup](https://rustup.rs/) to install the nightly toolchain with the required components:

   ```powershell
   rustup toolchain install nightly-2026-04-16 --component rustc-dev llvm-tools-preview rustfmt clippy
   rustup default nightly-2026-04-16
   ```

   The `rust-toolchain` file in the repository root will pin the correct nightly automatically once the toolchain is installed, but running the command above ensures the required components are present.

2. **Install Dylint** (the lint driver this project depends on):

   ```powershell
   cargo install cargo-dylint dylint-link --version "^6.0.1" --locked
   ```

3. **Clone the repository and build:**

   ```powershell
   git clone https://github.com/Tollcraft/soroban-cost-linter.git
   cd soroban-cost-linter
   cargo build
   ```

4. **Run the full quality checks** (these must pass before opening a PR):

    ```bash
    make check
    ```

    This runs `fmt-check` (`cargo fmt --all -- --check`), `lint` (`cargo clippy --workspace --all-targets -- -D warnings`), `test` (`cargo test --workspace`), and `check-docs` (`cargo run -p generate-lint-docs -- --check`) matching CI gates. See the [`Makefile`](./Makefile) for all available targets.

### 3. Adding a New Lint
- Read the [Scope: Clippy vs. soroban-cost-linter](./docs/scope_boundary.md) guide first. If a pattern is already covered by a Clippy lint and the Soroban cost story does not change the analysis, do not duplicate it here.
- Find a structural anti-pattern in Soroban that is input-independent and costly, and that is **not** already covered by a Clippy lint with the same cost-relevant semantics.
- Assign the lint to one of the five [lint categories](docs/lint_categories.md) and add it to the `LINT_METADATA` registry in `soroban_cost_lints/src/lib.rs`.
- Add a UI test fixture in `soroban_cost_lints/ui/<lint_name>.rs` with its blessed `.stderr` output (covering both positive triggering cases and negative non-triggering cases).
- Implement the lint using the `dylint` framework, checking the AST or HIR for the specific pattern.
- Update the `LINT_METADATA` entry with the correct category.
- Measure linter overhead with `cargo bench -p cargo-cost-lint`.
- Run `cargo run -p generate-lint-docs` from the workspace root to regenerate `docs/lints/README.md` and `docs/lints/lint-registry.json`.
- Run `make sync-lint-data` to refresh the snapshot in `cargo-cost-lint/lint-data/` that a packaged `cargo-cost-lint` builds from.

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

All PRs are checked by CI (Linux and Windows), and these checks must pass before a PR can be merged. Run them locally before pushing with `make check`:

```bash
make check
```

Or run the individual commands:

1. Check formatting with rustfmt (or run `make fmt` to automatically fix formatting):

   ```bash
   cargo fmt --all -- --check
   ```

2. Make sure Clippy passes with no warnings:

   ```bash
   cargo clippy --workspace --all-targets -- -D warnings
   ```

3. Make sure the test suite passes:

   ```bash
   cargo test --workspace
   ```

4. Make sure documentation is up to date:

   ```bash
   cargo run -p generate-lint-docs -- --check
   ```

> **Windows tip:** All commands above work identically in PowerShell. You can also run them in Git Bash if you prefer a Unix-like shell.

Follow the patterns already used in the codebase: `soroban_cost_lints` uses edition 2024, so prefer let-chains (`if let ... && let ...`) over nested `if let` blocks, and match the structure of the existing lint passes when adding a new lint.

### 4.1 Security and Policy Checks (`cargo-deny`)

This project uses `cargo-deny` in CI via the `cargo-deny` job in [`.github/workflows/security.yml`](./.github/workflows/security.yml) (all on `ubuntu-latest`) to enforce dependency policies. The policy is defined in [`deny.toml`](./deny.toml).

**When the check runs** (`security.yml:1-25`):
- Path-filtered `push` to `main` (triggers on `**/Cargo.lock`, `**/Cargo.toml`, `deny.toml`, `.github/workflows/security.yml`)
- Daily schedule at 06:00 UTC (`security.yml:20`) to catch newly disclosed vulnerabilities within 24 hours
- Manual `workflow_dispatch` from the Actions tab (`security.yml:24`)
- Not a `pull_request` trigger: the `Lint` job's `build-and-test` is the single check a PR waits on

**What the policy enforces** (`deny.toml`):
- **Licenses** (`[licenses]`): The exhaustive `allow` list is `MIT`, `Apache-2.0`, `Unicode-3.0`, `Apache-2.0 WITH LLVM-exception`, and `BSD-3-Clause` (`deny.toml:42-57`). Anything not listed is denied. Copyleft licenses (GPL/AGPL/LGPL/MPL-2.0) are rejected by virtue of not appearing on the list.
- **License exceptions** (`[licenses].exceptions`): `clippy_utils` (from a pinned git rev of rust-clippy) is explicitly allowed `MIT`/`Apache-2.0` because its license file may not be detectable by cargo-deny from a git source (`deny.toml:62-67`). This exception can be removed if clippy_utils is published to crates.io.
- **Advisories** (`[advisories]`): `yanked = "deny"` (`deny.toml:27`) and `unmaintained = "workspace"` (`deny.toml:24`). Vulnerability, unsound, and notice advisories always error (the former `vulnerability`/`unsound`/`notice` level keys were removed from the cargo-deny schema; `deny.toml:18-21` explains this).
- **Sources** (`[sources]`): `unknown-registry = "deny"` and `unknown-git = "deny"` (`deny.toml:80-81`). Git sources are restricted to `https://github.com/rust-lang/rust-clippy` (`deny.toml:82-86`).
- **Bans** (`[bans]`): `multiple-versions = "warn"` (`deny.toml:93`). Duplicate versions trigger a warning, not an error.

**Exceptions:**
If you need an exception (e.g., to ignore a specific advisory or allow a license for a specific crate), you can propose adding it to `deny.toml`. Any such exception must include a comment explaining why it is safe for this project and under what condition it can be removed.

**Running locally:**
Run the check locally before pushing to catch policy violations early:
```bash
cargo deny check
```

### 5. Upgrading the Nightly Toolchain

The pinned nightly is declared once in `rust-toolchain` (the single source of truth) and must stay in sync across multiple files. Upgrading is a multi-step, order-dependent process. The complete procedure—including identification of the matching `clippy_utils` revision, common breakages, and how to verify success—is documented in the [Nightly Upgrade Runbook](./docs/NIGHTLY_UPGRADE_RUNBOOK.md).

**TL;DR:** The critical relationship is between the nightly channel in `rust-toolchain` and the `clippy_utils` git revision in `soroban_cost_lints/Cargo.toml`. As of the current pin, `clippy_utils` rev `f9c23c1fcb96fb42e8706fa9c61043dc70423ba9` (Clippy 0.1.97) maps to `nightly-2026-04-16`; the mapping is recorded next to the dependency in `soroban_cost_lints/Cargo.toml`. See the runbook for guidance on finding the matching revision from `rust-lang/rust-clippy`'s `master` branch.

1. Update `rust-toolchain` with the new nightly date (e.g. `nightly-2026-04-16`).
2. Find the matching `clippy_utils` commit from the [`rust-lang/rust-clippy`](https://github.com/rust-lang/rust-clippy) repository's `master` branch, dated on or before the target nightly, and update the `rev` field in `soroban_cost_lints/Cargo.toml` (keeping the recorded nightly link comment in sync).
3. Update `.github/workflows/lint.yml`, `action.yml` (the `toolchain` input default), and `docs/integration.md` with the new nightly date.
4. Run the drift guard to confirm everything agrees:
   ```bash
   # Linux / macOS
   bash .github/scripts/validate-toolchain-pins.sh

   # Windows (PowerShell)
   powershell -ExecutionPolicy Bypass -File .github/scripts/validate-toolchain-pins.ps1
   ```
5. Run the full test suite:
   ```bash
   cargo test --workspace
   ```

A new lint is released as part of the next project version after its implementation has been merged. The release process is documented in [Releasing a New Lint](docs/release_process.md).

Before a release, maintainers must confirm that the lint is complete, documented, registered, covered by UI tests, and included in the changelog. The lint name must remain stable after release because it is part of the public configuration and command-line interface.

{% hint style="info" %}
The `Dockerfile` reads `rust-toolchain` at build time, so updating the channel there is sufficient — the container image will be rebuilt and published automatically by CI when `rust-toolchain` changes.
{% endhint %}

### 6. Submitting a Pull Request
### 7. Submitting a Pull Request
- Ensure your PR targets the `main` branch.
- Make sure the checks in the section above (`cargo fmt`, `cargo clippy`, `cargo test`) all pass.
- Provide a clear description of what the lint does and why it saves costs.
- If your pull request includes a user-visible change, add an appropriate entry under the **Unreleased** section of `CHANGELOG.md`. The entry will be moved into the next versioned release when a release is cut.
