# Releasing a New Lint

This guide describes how maintainers release a new lint in `soroban-cost-linter`. A lint is not ready for release until its implementation, tests, documentation, registry metadata, and changelog entry are all complete.

## 1. Confirm release readiness

Before preparing a release, verify that the lint:

- Has a stable lowercase `snake_case` name.
- Is registered in `soroban_cost_lints/src/lib.rs` and assigned to the correct cost category.
- Has UI coverage in the `soroban_cost_lints/ui` test suite, including the expected diagnostic output and non-triggering cases where appropriate.
- Has a lint reference page in `docs/lints/`.
- Is listed in `docs/lints/README.md`, the project `README.md`, and `soroban_cost_lints/README.md` where applicable.
- Explains the Soroban cost impact and, when useful, includes a compliant alternative.
- Does not duplicate a Clippy lint covered by the project's scope rules.
- Has no unexpected findings in the real-world corpus.

Lint names are part of the public interface. Once released, do not rename or remove a lint without treating the change as a compatibility change and documenting it explicitly.

## 2. Validate the change

Run the same checks used by CI from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

If the lint changes findings in the real-world corpus, review the new findings rather than blindly updating the baseline. Only regenerate the baseline after confirming that each new finding is expected:

```bash
BLESS=1 cargo test --test real_world_corpus --workspace
```

Review the resulting baseline change before committing it.

## 3. Update the changelog

Add a user-facing entry under the `Unreleased` section of `CHANGELOG.md`. The entry should include:

- The lint name.
- The pattern it detects.
- The Soroban resource or cost category affected.
- A short description of the recommended alternative, if one is available.

Keep the entry in `Unreleased` until the release version is known. Do not describe an unreleased lint as available in a versioned section.

## 4. Prepare the versioned release

When the release is approved:

1. Review all changes since the previous release and confirm that the new lint and its documentation are included.
2. Update the package version according to the repository's versioning policy. Keep related package versions consistent when they are released together.
3. Move the relevant `CHANGELOG.md` entries from `Unreleased` into a new versioned section with the release date.
4. Run the complete validation commands again after changing the version or changelog.
5. Commit the release preparation changes and create the release tag using the repository's established tag format.
6. Publish the release through the repository's configured release workflow or hosting process.

Do not publish a lint before its documentation and expected diagnostics are present in the same release. Downstream users should be able to discover the lint name, understand its diagnostic, and determine how to configure or suppress it from the released documentation.

## 5. Verify the release

After the release is published:

- Confirm that the release notes mention the new lint and link to its documentation.
- Confirm that the released source and documentation contain the same lint name and diagnostic behavior.
- Verify the installation or integration path used by downstream users.
- Check that the release workflow completed successfully.
- Open a follow-up issue for any release-only problem instead of silently changing the released lint name or behavior.

For pull requests that introduce a new lint, include `Closes #[issue number]` in the PR description when an issue is being resolved.
