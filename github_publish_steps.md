# GitHub Publish Steps — run these yourself

Every remaining GitHub-side item from `submission_checklist.md`, as copy-paste commands.
Run them in order from the repo root after `gh auth login` (in your own terminal).

## 1. Commit and push the local edits

Three files changed locally: `submission.md` (hype word removed, dead docs placeholder
removed), `README.md` (broken crates.io badge removed), plus the new
`submission_checklist.md` and this file.

```bash
git add submission.md README.md submission_checklist.md github_publish_steps.md
git commit -m "docs(submission): remove placeholder and broken badge, add pre-submission checklist"
git push origin main
```

## 2. Create the v0.1.0 tag and release

The repo currently has no tags — the release link in `submission.md` is dead until this runs.

```bash
git tag -a v0.1.0 -m "v0.1.0 — initial MVP release"
git push origin v0.1.0

gh release create v0.1.0 \
  --title "v0.1.0 — Initial MVP" \
  --notes "First release of soroban-cost-linter, a dylint-based static analysis tool for Soroban smart contracts.

## Included lints
- \`soroban_storage_in_loop\` — flags storage read/write operations inside loop bodies
- \`redundant_env_clone\` — detects unnecessary \`.clone()\` calls on \`Env\`
- \`unnecessary_host_function_call\` — flags repeated host function calls that should be bound once

## Usage
Requires \`cargo-dylint\` and \`dylint-link\`. See the [docs site](https://tollcraft.github.io/soroban-cost-linter/) for setup and CI integration."
```

Adjust the lint list to match what is actually implemented at tag time.

## 3. Add repository topics

```bash
gh repo edit Tollcraft/soroban-cost-linter \
  --add-topic stellar --add-topic soroban --add-topic dylint \
  --add-topic static-analysis --add-topic rust --add-topic linter
```

## 4. Create the planned issues

The script already exists and follows the required format (Summary / Acceptance Criteria /
Tech Stack, labels). Verified: the repo currently has 0 open issues.

```bash
# Labels must exist before the script can apply them:
gh label create lint --color 1D76DB --description "New or improved lint rule" 2>/dev/null || true
bash scripts/generate_issues.sh
```

## 5. Branch protection on main

The verified CI job name is `lint` (from .github/workflows/lint.yml).

```bash
gh api -X PUT repos/Tollcraft/soroban-cost-linter/branches/main/protection \
  --input - <<'EOF'
{
  "required_status_checks": { "strict": true, "contexts": ["lint"] },
  "enforce_admins": false,
  "required_pull_request_reviews": { "required_approving_review_count": 1 },
  "restrictions": null
}
EOF
```

(Or via web UI: Settings → Branches → Add rule → require PRs, 1 approval, status check `lint`.)

## 6. Before submitting

- Record the demo video per `demo_script.md` and put the real link in `submission.md`.
- Re-check the approved list live: https://www.drips.network/wave/stellar/repos
- Click every link in `submission.md` from a logged-out browser.
