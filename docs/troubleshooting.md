# Troubleshooting

This page is organised by the symptom you actually see. If **this** happens, the
fix is **that**. Use the "Confirm" command before applying any fix — it tells you
whether the diagnosis is right without guessing.

Every symptom here is one the repository has hit or documented itself. The
common thread is that this is a Dylint-based tool, and Dylint tends to **fail
quietly**: misconfigure an install step and you rarely get an error — you get a
run that reports zero findings.

{% hint style="info" %}
Not sure whether the tool works at all? Start with
[How do I tell whether it is working?](#how-do-i-tell-whether-it-is-working). It
is a three-line contract with a known-bad pattern and the exact output to expect,
and it is the fastest way to rule out an install problem before chasing a
contract issue.
{% endhint %}

## How do I tell whether it is working?

Put this in any Rust file in a Soroban contract workspace (or copy
`soroban_cost_lints/ui/soroban_storage_in_loop.rs` from this repo, which is a
self-contained fixture that needs no SDK import):

```rust
pub fn write_in_loop(env: &Env, n: u32) {
    for i in 0..n {
        env.storage().instance().set(&i, &1); // known-bad
    }
}
```

Run it:

```bash
cargo cost-lint
```

No flags needed: `soroban_storage_in_loop` is `deny` by default, so a
storage-in-loop finding is reported as an `error` and the run exits `1`. A working
install reports the finding, names the lint, and fails:

```text
error: storage operation inside a loop
  --> src/lib.rs:12:9
   |
LL |         env.storage().instance().set(&i, &1);
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: move storage operations out of the loop or accumulate mutations in memory first
   = note: `#[warn(soroban_storage_in_loop)]` on by default
```

What to look for:

- A `soroban`-named lint diagnostic, not zero findings.
- The exit code is **non-zero** (the deny-level finding fails the run).
- The output names the file, line, and column.

If instead the command prints nothing relevant or exits `0` with no findings on
this known-bad pattern, the lints are **not being loaded at all** — go to
[The linter runs and reports nothing on code that should trigger](#the-linter-runs-and-reports-nothing-on-code-that-should-trigger).

---

## The linter runs and reports nothing on code that should trigger

**Symptom:** `cargo cost-lint` completes quickly, exits `0`, and prints no
findings — even on code you are sure is expensive.

**Cause:** The lint library was not loaded, so the compiler never ran any of the
passes. This tool shells out to `cargo dylint rustc …` and parses its JSON
diagnostics; if no lint library is loaded, there is nothing to parse and the run
"passes" with zero findings. `cargo-cost-lint` does not fail the run when Dylint
loads nothing.

**Confirm** the inventory is populated before anything else:

```bash
cargo cost-lint --list-lints
```

Without `--format json` this prints a human-readable inventory under a
`Lint inventory (version 1.0):` banner listing lints such as
`soroban_storage_in_loop`, `redundant_env_clone`, `unnecessary_host_function_call`
(`--format json` gives the same data as JSON). Importantly, listing works without
the lint library loaded, so it isolates a CLI/build problem from a library-load
problem.

- If the inventory is **empty** or the command errors, the CLI build failed to
  find its registered lints — see the causes below.
- If the inventory is **present**, the toolchain and CLI are fine, so the zero
  findings are almost always a **build / library-load** problem:
  - [cargo-dylint cannot find the lint library](#cargo-dylint-cannot-find-the-lint-library).
  - [Missing rustc-dev / llvm-tools-preview](#missing-rustc-dev--llvm-tools-preview).
  - [Toolchain mismatch against the pinned nightly](#toolchain-mismatch-against-the-pinned-nightly).
  - [cargo-dylint / dylint-link not installed or the wrong version](#cargo-dylint--dylint-link-not-installed-or-the-wrong-version).

**So the rule is:** a "runs but reports nothing" result is almost never a bug in
your contract — it is a setup problem that is hiding behind an exit code of `0`.

---

## cargo dylint cannot find the lint library

**Symptom:** something like `` Could not find `--lib soroban_cost_lints` `` when the
lint library is built, or `cargo cost-lint` silently finds no lints (see above).

**Cause:** Dylint locates a library by a filename that embeds the toolchain:
`lib<name>@<toolchain>.so` (for example
`libsoroban_cost_lints@nightly-2026-04-16-x86_64-unknown-linux-gnu.so`). A plain
`cargo build` produces `libsoroban_cost_lints.so` — **no `@` suffix** — which
Dylint does not match. The `@` suffix is only added when the cdylib is linked
through `dylint-link` (the repo does this via
`soroban_cost_lints/.cargo/config.toml`, which sets
`rustflags = ["-C", "linker=dylint-link"]`).

**Confirm:**

```bash
# From the repo root, after building:
ls -1 target/{debug,release}/*soroban_cost_lints*.so
```

- ✔ Working: a file named `libsoroban_cost_lints@<toolchain>.so`.
- ✘ Broken: only `libsoroban_cost_lints.so` (no `@`).

**Fix:** build the cdylib *from inside* `soroban_cost_lints/` so Cargo picks up
its `dylint-link` configuration — do not point `cargo build` at it from the
workspace root with a plain manifest path:

```bash
cd soroban_cost_lints
cargo build
# then run from your contract workspace:
DYLINT_LIBRARY_PATH="$PWD/target/debug" cargo cost-lint
```

The `GitHub Action succeeds without linting` entry below is this same bug showing
up green in CI.

---

## Toolchain mismatch against the pinned nightly

**Symptom:** a confusing compile error while building the lints, or the lint
library not being found even though `dylint-link` was used.

**Cause:** The project pins one nightly (`nightly-2026-04-16`) in
`rust-toolchain`, and Dylint + the lints are tightly coupled to that exact
compiler. On a different toolchain the build fails or the library gets stamped
with a different `@<toolchain>` suffix and is then not found.

The pin is a **single source of truth** (`rust-toolchain`), mirrored into
`action.yml`, `.github/workflows/*.yml`, `templates/github-action.yml`,
`docs/integration.md`, `CONTRIBUTING.md`, `README.md`, and `docs/windows_setup.md`.
Drifting any one of them out of sync is enough to break the build.

**Confirm:**

```bash
rustup toolchain list
cat rust-toolchain            # channel = "nightly-2026-04-16"
```

Build errors that point at the compiler or the `clippy_utils` revision look like:

```text
error[E0425]: cannot find function `get_enclosing_block` in module `clippy_utils`
error: no field `kind` on type `Expr`
error: the feature `rustc_private` is not available in this edition
```

(`clippy_utils` is pinned to a commit dated for the pinned nightly; a mismatched
pair is the most common cause of these.)

**Fix:** use the pinned nightly and keep every pin in sync:

```bash
rustup toolchain install nightly-2026-04-16 --component rustc-dev llvm-tools-preview rustfmt clippy
# In this repo, enforce that every pin agrees:
bash .github/scripts/validate-toolchain-pins.sh
```

---

## Missing rustc-dev / llvm-tools-preview

**Symptom:** building the lints fails with errors about missing crates or a
forbidden `#![feature]`:

```text
error[E0463]: can't find crate for `rustc_span`
  = help: maybe you need to install the missing components with:
            `rustup component add rust-src rustc-dev llvm-tools-preview`

error[E0554]: `#![feature]` may not be used on the stable release channel
```

**Cause:** The lints use `#![feature(rustc_private)]` and depend on `rustc_*`
crate internals. Those are only available when the `rustc-dev` component (and the
`llvm-tools-preview` component that Dylint uses to load the driver) are installed
for the active toolchain. `rustc_span` etc. are not provided by the plain `rustc`
binary. (If you instead see **`E0554`** — `#![feature]` on the stable channel — you
are on the wrong toolchain entirely, not just missing a component; see
[Toolchain mismatch against the pinned nightly](#toolchain-mismatch-against-the-pinned-nightly).)

**Confirm:**

```bash
rustup component list --toolchain nightly-2026-04-16 --installed
# must include rustc-dev-x86_64-unknown-linux-gnu and llvm-tools-x86_64-unknown-linux-gnu
```

**Fix:**

```bash
rustup component add rustc-dev llvm-tools-preview --toolchain nightly-2026-04-16
```

---

## cargo-dylint / dylint-link not installed or the wrong version

**Symptom:** `cargo cost-lint` fails with a "not installed" message, or building
the lint library fails with a linker error mentioning `dylint-link`.

**Cause:** These are two separate binaries and both must be on `PATH`:

- `cargo-dylint` — the cargo subcommand that runs the compiler with the lint
  driver.
- `dylint-link` — the linker wrapper that renames the cdylib to the
  `lib<name>@<toolchain>.so` filename Dylint resolves.

If `cargo-dylint` is missing, the friendly error the CLI is wired to show is:

```text
error: `cargo-dylint` is not installed.
To install it, run:
    cargo install cargo-dylint dylint-link --version "^6.0.1"
```

But because `cargo-dylint` is a cargo *subcommand*, a missing binary can also just
make `cargo dylint …` produce a "no such subcommand" error that the CLI does not
recognise — which surfaces again as **zero findings** rather than an error. And at
the wrong version, `dylint-link` either does not exist or renames the library
wrongly.

**Confirm:**

```bash
cargo-dylint --version
dylint-link --version
# Both must resolve to a 6.x that satisfies the project pin "^6.0.1"
```

**Fix:** install the pinned version:

```bash
cargo install cargo-dylint dylint-link --version "^6.0.1" --locked
```

If you only have one binary, or the versions disagree, reinstall both together as
above — the version pin is what keeps the `@<toolchain>` naming consistent.

---

## The GitHub Action succeeds without linting anything

**Symptom:** the `Soroban Cost Lint` workflow reports **green**, but no annotations
appear on the PR and no `soroban_*` findings are reported.

**Cause (the silent one):** the composite action builds the lint cdylib, registers
the toolchain, and then runs `cargo-cost-lint --format github`. If that build did
**not** go through `dylint-link`, the library lands as `libsoroban_cost_lints.so`
with no `@` suffix, the "Run Cost Linter" step finds no library, and the job exits
`0` — exactly the "runs and reports nothing" case, but masquerading as a passing
CI check.

**Confirm:** check the "Run Cost Linter" step log in the workflow run.

- ✔ Working: you see `::error file=...` / `::warning file=...` annotation lines
  (or a clean no-findings run).
- ✘ Broken: a run that supposedly "succeeds" but produced no annotations — the
  library was not loaded.

This is the exact regression documented in the action's own
[`runs` definition](../action.yml) (comment: a plain `cargo build` produced
`libsoroban_cost_lints.so` with no toolchain suffix, and the "Run Cost Linter"
step could not find it).

**Also note:** by default most lints are `warn`, and `warn`-only findings **annotate
the diff but do not fail the job**. The one exception is `soroban_storage_in_loop`,
which defaults to `deny`. So a green job with a couple of `::warning::` annotations
is not the bug above — it is working as configured. If you want other lints to
fail CI, raise them to `deny` in your `budget.toml` (see
[How do I tell whether it is working?](#how-do-i-tell-whether-it-is-working) and the
[Integration Guide](integration.md#github-actions)).

---

## Still stuck?

- Re-run with the machine-readable inventory to separate "not loaded" from
  "no findings":

  ```bash
  cargo cost-lint --list-lints --format json
  ```

- Confirm the wire path the tool actually takes: `cargo dylint rustc …` needs
  `cargo-dylint`, a matching pinned nightly, its two toolchain components, and a
  `dylint-link`-linked `libsoroban_cost_lints@<toolchain>.so`. Any of these being
  off produces the **same** surface symptom: a run that reports nothing.

- For Windows-specific dynamic-loading failures (`error: unloaded library`, MSVC
  `LNK1104`), see the [Windows Setup Guide](windows_setup.md#common-windows-issues).
