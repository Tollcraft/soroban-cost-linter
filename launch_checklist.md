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
- [x] **Branch protection on `main`** — PRs required with 1 approving review.

## Remaining

- [ ] **Record the demo video** (1–2 min) following `demo_script.md`: broken contract →
      `cargo cost-lint` fails with non-zero exit → fix → clean pass. Link it from the README.
- [ ] **Require the `lint` status check on `main`** — branch protection currently has no
      required status contexts:

      ```bash
      gh api -X PATCH repos/Tollcraft/soroban-cost-linter/branches/main/protection/required_status_checks \
        -F strict=true -f "contexts[]=lint"
      ```

- [ ] **Publish to crates.io** (optional) — once published, restore the crates.io badge
      in the README.
- [ ] **Local dev sanity check** — confirm the `nightly-2026-04-16` toolchain is fully
      installed and `cargo test --workspace` passes before recording the demo.
