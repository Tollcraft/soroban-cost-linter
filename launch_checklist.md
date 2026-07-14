# Launch Checklist

Remaining work to make the `v0.1.x` launch complete. Live state verified against the
GitHub API on 2026-07-14.

## Done

- [x] **Docs site is live** — https://tollcraft.gitbook.io/docs (GitBook, configured via
      `.gitbook.yaml` with content in `docs/`; also set as the repo homepage)
- [x] **CI is green on `main`** — the `Lint` workflow passes; the job name for
      branch-protection status checks is `lint`.
- [x] **Release published** — tags `v0.1.0` and `v0.1.1` exist, with a
      "v0.1.0 — Initial MVP" GitHub release.
- [x] **Roadmap issues open** — 14 open issues generated via `scripts/generate_issues.sh`.
- [x] **Repo topics set** — `stellar`, `soroban`, `dylint`, `static-analysis`, `rust`, `linter`.
- [x] **Branch protection on `main`** — PRs required with 1 approving review, plus the
      `lint` status check (strict) required since 2026-07-14.
- [x] **Demo recording** — https://asciinema.org/a/1DpqHMqqOOXoZzMI (asciinema, per
      `demo_script.md`: lint failure blocks the build, fix, clean pass), linked from the
      README header.
- [x] **Local dev sanity check** — `nightly-2026-04-16` toolchain installed;
      `cargo test --workspace` passes (UI tests green, 2026-07-14).

## Remaining

- [ ] **Publish to crates.io** (optional) — once published, restore the crates.io badge
      in the README.
