# Pre-Submission Checklist — Drips Wave 7

Derived from the Stellar Wave Builder master prompt (Phases 10–12). Remote state was
verified against the GitHub API and live URLs on 2026-07-03.

## Verified done

- [x] **Docs site is live** — https://tollcraft.github.io/soroban-cost-linter/ returns 200;
      the Pages deploy workflow succeeded on `main`.
- [x] **CI is green on `main`** — latest `Lint` workflow run: success. The job name for
      branch-protection status checks is `lint`.
- [x] **One-paragraph plain-English description** in `submission.md` (hype word removed,
      concrete CI mechanism stated).
- [x] **README badge cleanup** — crates.io badge removed (crate is not published; the
      badge rendered as broken/nonexistent to reviewers).

## Blocking — submission reads as unfinished without these

- [ ] **Record the demo video** (1–2 min) following `demo_script.md`: broken contract →
      `cargo cost-lint` fails with non-zero exit → fix → clean pass. Upload and replace
      the placeholder link in `submission.md`.
- [ ] **Create the `v0.1.0` tag and release** — verified via API: the repo has **no tags
      and no releases**, so the release link in `submission.md` is currently dead.
      Release body should state what the release contains and the supported lints.
- [ ] **Confirm the project is not already in the approved list** — fetch
      https://www.drips.network/wave/stellar/repos live on submission day.

## Needs GitHub auth (`gh auth login`) or web UI

- [ ] **Run `scripts/generate_issues.sh`** — verified via API: **0 open issues** exist.
      The "planned issues" section of the submission must point at real, open issues.
- [ ] **Add GitHub topics** — verified via API: the topics list is **empty**. Suggested:
      `stellar`, `soroban`, `dylint`, `static-analysis`, `rust`, `linter`.
- [ ] **Branch protection on `main`** — require PRs, at least one approval, and the
      required status check `lint` (verified job name from `.github/workflows/lint.yml`).
- [ ] **Publish to crates.io** (optional) — the crate does not exist there yet. If
      published later, restore the crates.io badge in the README.

## Submission form content

- [ ] **"Planned issues" description** — write it from the real issues once created,
      grouped by area (new lints, `budget.toml` config, CI action, docs).
- [ ] **Repo relationship description** — one or two sentences on how this pairs with
      `soroban-budget-assert` (static Stage 1 vs runtime Stage 2, shared `budget.toml`),
      if both repos are submitted.
- [ ] **Final link sweep** — click every link in `submission.md` from a logged-out
      browser: repo, release tag, docs site, demo video, issue tracker.

## Local environment (for the demo recording)

- [ ] **Repair verified in progress** — the local `nightly-2026-04-16` toolchain was
      partially installed; it is being reinstalled and `cargo test --workspace` run to
      confirm the linter works locally before recording the demo.
