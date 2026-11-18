## Description
<!-- Describe your changes in detail -->

## Related Issue
<!-- If this PR addresses an existing issue, please link it here. -->
Closes #

## Motivation and Context
<!-- Why is this change required? What problem does it solve? -->

## How Has This Been Tested?
<!-- Please describe in detail how you tested your changes. -->
- [ ] Passed `cargo fmt --all -- --check`
- [ ] Passed `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] Passed `cargo test --workspace`
- [ ] _If you added or changed a lint:_ Regenerated UI fixtures with `UPDATE_EXPECT=1 cargo test --workspace` and reviewed every changed `.stderr` file
- [ ] _If you added or changed a lint:_ Regenerated baseline with `BLESS=1 cargo test --test real_world_corpus --workspace`
- [ ] _If you added or changed a lint:_ Added a CHANGELOG.md entry under the `[Unreleased]` section

## Types of changes
<!-- What types of changes does your code introduce? -->
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update
