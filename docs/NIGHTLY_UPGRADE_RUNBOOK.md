# Nightly Toolchain Upgrade Runbook

Upgrading the pinned Rust nightly is a multi-file, order-dependent operation. This runbook documents the procedure, common issues, and how to verify success.

## Why This Matters

The project depends on `rustc` internals—specifically `rustc_hir`, `rustc_middle`, `rustc_session`—which change frequently between nightly releases. To keep up with Rust development and receive new compiler features and bug fixes, the nightly pin must be upgraded periodically. A mismatched `clippy_utils` revision can introduce API incompatibilities that are difficult to debug without knowing the relationship between them.

## The Single Source of Truth

The nightly channel is declared once in [`rust-toolchain`](../rust-toolchain). This is the source of truth. All other files—workflows, templates, documentation—should reflect this same date.

**Note on #104:** Issue #104 proposes making the `clippy_utils` revision single-sourced from the nightly date (automatically rather than manually). When that lands, this runbook will simplify significantly. For now, you must update them together.

---

## Upgrade Procedure

### Step 1: Identify the Target Nightly

Upgrade to a nightly published in the last 1–2 weeks, not an arbitrary future date. Rust's nightly channel publishes once per day, and older nightlies eventually become unavailable in toolchain registries.

A reasonable upgrade target is something like `nightly-2026-05-15` if today is late May 2026 and the current pin is `nightly-2026-04-16`.

### Step 2: Find the Matching `clippy_utils` Revision

This is where most mismatches happen. The `clippy_utils` crate lives in the `rust-clippy` repository, and the revision pin must match a commit that was published on (or before) your target nightly date.

**Procedure:**

1. Open the [rust-clippy repository](https://github.com/rust-lang/rust-clippy) on GitHub.

2. The historical `rustup` branch (which used to carry one commit per nightly) has been retired. Use the `master` branch instead.

3. On `master`, view the commit history of the repository root:

   - Example URL: `https://github.com/rust-lang/rust-clippy/commits/master/`

4. Find the most recent commit dated **on or before** your target nightly. For `nightly-2026-05-15`, look for a commit dated **May 15, 2026** (or the closest date prior). A clippy commit from that day is the closest available proxy for the clippy bundled into that nightly.

5. Click into that commit and copy its SHA (the long hexadecimal string):

   ```
   f6d310692116e9a527ce6d0b3526c965d9c5d7b9   <- This is the rev
   ```

**Why `master` (and not a dedicated sync branch)?**

The `rustup` branch used to hold commits synchronized with `rustc` releases; it no longer exists. `master` moves faster and independently, so a commit picked by date is only an approximation. If the resolved rev fails to compile against the target nightly (see [Common Breakages](#common-breakages)), nudge it by a day or two and retry.

> **Automation note:** The scheduled workflow (see [Automated Nightly Bump](#automated-nightly-bump)) resolves this rev for you via the GitHub API: it requests the latest `master` commit dated on or before the target nightly. You only do this by hand when reviewing/override.

### Step 3: Update `rust-toolchain`

Open [`rust-toolchain`](../rust-toolchain) and update the `channel` field:

```toml
[toolchain]
channel = "nightly-2026-05-15"  # <- New date
components = ["llvm-tools-preview", "rustc-dev", "rustfmt", "clippy"]
```

### Step 4: Update `soroban_cost_lints/Cargo.toml`

Open [`soroban_cost_lints/Cargo.toml`](../soroban_cost_lints/Cargo.toml) and update the `clippy_utils` dependency:

```toml
[dependencies]
clippy_utils = { git = "https://github.com/rust-lang/rust-clippy", rev = "YOUR_NEW_REV_HERE" }
dylint_linting = "6.0.1"
openssl = { version = "0.10", features = ["vendored"] }
```

Replace `YOUR_NEW_REV_HERE` with the SHA you copied from step 2.

### Step 5: Update Workflow Files

Update the nightly pin in two CI workflow files. Both should match the channel you set in `rust-toolchain`.

**File 1: `.github/workflows/lint.yml`**

```yaml
- name: Install Rust
  uses: dtolnay/rust-toolchain@master
  with:
    toolchain: nightly-2026-05-15  # <- Update this
    components: rustc-dev, llvm-tools-preview, rustfmt, clippy
```

**File 2: `templates/github-action.yml`**

```yaml
- name: Install Rust
  uses: dtolnay/rust-toolchain@master
  with:
    toolchain: nightly-2026-05-15  # <- Update this
    components: rustc-dev, llvm-tools-preview
```

### Step 6: Update Documentation

Update the example in [`docs/integration.md`](../docs/integration.md):

Look for the section mentioning the toolchain pin and update the date there as well. (Example reference: search for the template's URL or the date in the file.)

### Step 7: Validate Pin Consistency

The repository includes a validation script to check that all pin references agree. Run:

```bash
bash .github/scripts/validate-toolchain-pins.sh
```

If any file is out of sync, the script will print an error naming the file, the mismatched value, and the expected one. **Do not proceed until all pins match.**

---

## Testing the Upgrade

### Step 8: Run the Full Test Suite

```bash
cargo test --workspace
```

This invokes the test suite under the new nightly. Watch for errors related to:

1. **`clippy_utils` API changes:** If a function signature changed or was removed, you'll see a compile error like:
   ```
   error[E0425]: cannot find function `get_enclosing_block` in module `clippy_utils`
   ```
   See [Common Breakages](#common-breakages) below for guidance.

2. **`rustc_hir` shape changes:** If the HIR structure changed, you might see:
   ```
   error: no field `kind` on type `Expr`
   ```
   The new nightly may have renamed or restructured HIR types.

3. **UI test failures:** If the compiler's error message format changed, expected output won't match. See [Blessing UI Tests](#blessing-ui-tests) below.

### Step 9: Blessing UI Tests

UI tests verify that lint diagnostics are emitted correctly. They store **expected** compiler output in `.stderr` files and compare against actual output when the tests run.

After a nightly upgrade, compiler diagnostic formatting sometimes changes (e.g., spans, colors, or message phrasing). The UI tests may fail even though the lints still work correctly. You must "bless" (re-approve) the output:

1. Run the UI test with the `BLESS` environment variable set:

   ```bash
   BLESS=1 cargo test --workspace
   ```

2. This regenerates all `.stderr` files to match the current nightly's output format.

3. Review the git diff carefully:

   ```bash
   git diff soroban_cost_lints/ui/
   ```

   Look for:
   - Line number changes (expected if spans shifted)
   - Message text changes (usually safe, but verify the meaning didn't change)
   - Missing or new warnings (may indicate a lint broke or a new one appeared)

4. If the changes look reasonable, stage and commit them:

   ```bash
   git add soroban_cost_lints/ui/main.stderr
   git commit -m "blessing: Update UI tests for nightly-2026-05-15"
   ```

---

## Common Breakages

### Breakage #1: `clippy_utils` API Changed or Was Removed

**Symptom:**

```
error[E0425]: cannot find function `get_enclosing_x` in module `clippy_utils`
```

**How to Fix:**

1. Check the [rust-clippy GitHub repository](https://github.com/rust-lang/rust-clippy) for the matching commit (the rev you picked in Step 2).
2. Search the commit's `clippy_utils/` directory for an alternative function name or migration guide in comments.
3. Look at recent clippy PRs or the file history for the breaking change. GitHub often links to issues or PRs in commit messages.
4. Update the lint code in `soroban_cost_lints/src/lib.rs` to use the new API.

**Example:** If `get_enclosing_block` was removed and replaced with `get_enclosing_block_or_loop`, update the import and call site.

### Breakage #2: `rustc_hir` Structure Changed

**Symptom:**

```
error: `Expr` has no field named `kind`; available fields: `hir_id`, `span`, ...
```

or

```
error: function or associated item `Path` not found
```

**How to Fix:**

1. The HIR structure likely evolved. Re-read the field names or enum variants in the new nightly's `rustc_hir` definition.
2. Use `rustc` docs or the source code to understand the new shape.
3. Update pattern matches and field accesses in the lint implementation.

**Example:** Older nightly might have `expr.kind`, but newer ones might use `expr.as_ref().kind` or a different accessor.

### Breakage #3: Required Crate Features or `feature(rustc_private)` Not Available

**Symptom:**

```
error: the feature `rustc_private` is not available in this edition
```

**How to Fix:**

1. Confirm that the feature is still enabled in `lib.rs`:
   ```rust
   #![feature(rustc_private)]
   ```
2. If it's enabled but the error persists, the nightly may have removed the feature. This is rare; check the nightly release notes.
3. If the issue is a different feature, search the lint code for the feature gate and update as needed.

### Breakage #4: Compiler Diagnostic Output Format Changed

**Symptom:**

UI tests fail, but the lints seem to fire correctly:

```
thread 'ui::tests' panicked at 'assertion failed: ... expected output != actual output'
```

**How to Fix:**

This is expected after a nightly upgrade. Follow the [Blessing UI Tests](#blessing-ui-tests) section above to re-bless the output.

---

## Verification Checklist

After completing all steps:

- [ ] `rust-toolchain` channel updated
- [ ] `soroban_cost_lints/Cargo.toml` clippy_utils rev updated
- [ ] `.github/workflows/lint.yml` toolchain pin updated
- [ ] `templates/github-action.yml` toolchain pin updated
- [ ] `docs/integration.md` documentation updated
- [ ] `validate-toolchain-pins.sh` passes without errors
- [ ] `cargo test --workspace` passes
- [ ] (If needed) UI tests blessed with `BLESS=1 cargo test --workspace`
- [ ] No new compiler warnings or errors introduced
- [ ] `cargo fmt --all` and `cargo clippy` pass

---

## Automated Nightly Bump

This repository ships a scheduled workflow, `.github/workflows/nightly-bump.yml`, that performs the *mechanical* part of this runbook on a timer (weekly, plus a manual `workflow_dispatch` trigger). It exists so the bump stops depending on someone remembering — turning an unbounded backlog into a small, regular one.

### What the workflow does

1. **Picks a target.** By default it targets a nightly from ~7 days ago (within the "last 1–2 weeks" window). You can override with the `target_nightly` input.
2. **Resolves the `clippy_utils` rev** for that date via the GitHub API (see [Step 2](#step-2-find-the-matching-clippy_utils-revision)). Override with the `clippy_rev` input if the auto-resolved rev is wrong.
3. **Applies the edit** across every file this runbook lists (`rust-toolchain`, `soroban_cost_lints/Cargo.toml`, the two workflows, `action.yml`, `templates/github-action.yml`, `docs/integration.md`, `CONTRIBUTING.md`, `README.md`, `docs/windows_setup.md`) in one go — the same set enforced by `validate-toolchain-pins.sh`.
4. **Runs the full test suite** (`cargo test --workspace`) under the new nightly.
5. **Opens or updates a single PR** on the `ci/nightly-bump` branch. It never merges.

### When the run passes

The PR's checks are green. A human reviews the `git diff` (especially any regenerated `.stderr` UI fixtures — confirm they are cosmetic) and merges. No new PR is created on the next run unless the pin has since drifted.

### When the run fails (this is the valuable case)

The PR is still opened/updated, and its body contains a **scope report** instead of just "it failed":

- **Compile errors** — the unique `error[EXXXX]` / `error:` messages, which usually point at `clippy_utils` / `rustc_hir` API breakage (Breakages #1 and #2).
- **Likely affected lints** — the UI fixtures that surfaced in the output, mapping each to a lint in `soroban_cost_lints/src/lib.rs`.
- **Failing tests** — the individual test paths that did not pass.

That report *is* the work this runbook exists to scope. Pick up where it leaves off:

1. Follow [Common Breakages](#common-breakages) to fix the code.
2. Push the fix to the `ci/nightly-bump` branch — the workflow reuses the same PR, so you do **not** get a new PR each run.
3. If the rev was resolved incorrectly, re-run the workflow with `clippy_rev` set to the correct SHA rather than hand-editing.

### Notes

- **No automatic merge.** The PR is always for a human to review, even when green.
- **One PR, reused.** Every run targets the same branch (`ci/nightly-bump`), so a failing run updates the existing PR instead of spamming new ones.
- **The script behind the workflow** lives at `.github/scripts/bump-nightly.sh` (with `resolve-clippy-rev.js` and `summarize-test-failures.js`). You can run `bash .github/scripts/bump-nightly.sh <nightly-YYYY-MM-DD>` locally to preview the edits it would make.

---

## If You Get Stuck

1. **Check the git history:** Look at previous nightly upgrade commits to see patterns:

   ```bash
   git log --grep="nightly" --oneline
   ```

2. **Review the rust-clippy repository:** The rust-clippy commits often note what changed. Start with the commit message of the rev you selected.

3. **Ask on the project Discord or Telegram:** The maintainers can help debug API changes or point you to resources.

---

## Future Simplification

Issue #104 proposes deriving the `clippy_utils` revision automatically from the nightly date. Once that lands, the number of manual update points will shrink, and this runbook's mechanical steps will simplify. For now, keep all four files (rust-toolchain, Cargo.toml, two workflows) in sync.

---

## Notes for Reviewers

When reviewing a PR that upgrades the nightly:

1. Verify that all four pin locations were updated to the same date.
2. Run the validation script locally to confirm consistency.
3. Spot-check the UI test diff to ensure no unexpected lints disappeared or were added.
4. Confirm that no new compiler warnings were introduced in the lint code.
5. If there are API changes, review the migrations in the lint code to ensure they are sound.
