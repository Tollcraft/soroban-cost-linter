# `soroban-cost-linter` Audit & Completion Roadmap

This document provides a comprehensive audit of the `soroban-cost-linter` project against the strict standards outlined in the **Stellar Wave Builder — Master System Prompt** (Drips Wave program). It identifies what is currently missing and provides a clear, actionable roadmap to bring the project to a submission-ready state.

---

## 1. Audit Findings: What is Missing?

Based on a review of the repository and the Drips Wave Master System Prompt, the following required components are missing or incomplete:

### Phase 10 — Repo Hygiene for Program Approval
*   **Missing `CONTRIBUTING.md`**: Required for outlining how external developers can contribute to the linting engine and AST parsing.
*   **Missing `SECURITY.md`**: Required for responsible disclosure contact information.
*   **Incomplete `README.md`**: The current README is functional but does not meet the "approved pattern" aesthetics:
    *   No project Banner/Logo.
    *   No status badges (Build, License, Version).
    *   Missing a Maintainer Table (with Telegram or contact info).
    *   Missing a Contributors Credits section (e.g., using `contrib.rocks`).
    *   No Community/Discord link.
*   **Missing Issue Generation Script**: Phase 10 requires a `gh` CLI script that generates all planned issues (with labels, summary, tech stack, and acceptance criteria checkboxes).

### Phase 11 — Documentation Site
*   **Missing Separate Docs Site**: A proper documentation site (e.g., GitBook or mdBook) is required, separate from the README. It must include:
    *   Introduction and problem statement (with real cited figures of Soroban storage costs).
    *   Lint Mechanics (how AST/HIR analysis works in this tool).
    *   Rule Reference (detailed guide on `soroban_storage_in_loop`, `redundant_env_clone`, etc.).
    *   Developer/Contributor Guide (local setup with Dylint).

### Phase 12 — Submission Requirements
*   **Missing Demo Video**: A short video showing the linter running in a terminal or IDE, catching a structural error, and blocking a CI pipeline.
*   **Release Tag**: No `v0.1.0` release tag exists yet.

### Core Tooling & Features (MVP Gaps)
*   **Missing Lints**: Only `soroban_storage_in_loop` is currently implemented (per `project_log.md`). `redundant_env_clone` and `unnecessary_host_function_call` are pending.
*   **Missing CLI Wrapper**: The roadmap mentions a `cargo-cost-lint` wrapper, but there is no CLI crate yet (users currently have to run raw Dylint commands).
*   **Missing `budget.toml` Integration**: Configuration parsing for workspace-level rule severity is not yet implemented.
*   **Missing CI/CD Templates**: No `.github/workflows/` provided to show users how to integrate the linter into their GitHub Actions.

---

## 2. Roadmap to Completion

Follow this sequenced roadmap to complete the project up to the Drips Wave standard.

### Step 1: Repo Hygiene & Documentation Baseline (Phase 10)
1.  **Create `CONTRIBUTING.md` and `SECURITY.md`**.
2.  **Upgrade `README.md`**:
    *   Design and add a project banner.
    *   Add badges (CI status, crates.io, license).
    *   Add a Maintainer table.
    *   Add `[![Contributors](https://contrib.rocks/image?repo=your-org/soroban-cost-linter)](https://github.com/your-org/soroban-cost-linter/graphs/contributors)`.
3.  **Write Issue Generation Script**: Create `scripts/generate_issues.sh` utilizing the `gh` CLI to create structured issues for the remaining lints, CLI wrapper, and docs. Execute it to populate the repo.

### Step 2: Core Feature Completion (MVP)
1.  **Develop CLI Wrapper**: Create a new crate in the workspace (`cargo-cost-lint`) that orchestrates the underlying Dylint execution so users have a clean UX (`cargo cost-lint`).
2.  **Implement Configuration Parsing**: Add logic to read `budget.toml` from the user's workspace root to dynamically set lint levels (`warn`, `deny`, `allow`).
3.  **Implement Remaining Lints**:
    *   `redundant_env_clone`
    *   `unnecessary_host_function_call`
4.  **Add CI/CD Workflows**: Create a `.github/workflows/lint.yml` to lint the linter itself, and a `templates/github-action.yml` for users to copy.

### Step 3: Documentation Site (Phase 11)
1.  **Initialize Docs**: Set up mdBook (common in Rust ecosystem) or GitBook in a `docs/` directory.
2.  **Write Content**:
    *   **Introduction**: Explain the financial cost of Soroban storage operations.
    *   **Lint Reference**: One page per lint explaining the anti-pattern, why it's bad, and the suggested fix.
    *   **Integration Guide**: How to set it up in CI/CD.

### Step 4: Submission Preparation (Phase 12)
1.  **Record Demo Video**: Record a 1-2 minute screencast showing a Soroban contract with a `storage().set()` in a loop, running `cargo cost-lint`, getting a failure, fixing the code, and getting a pass.
2.  **Cut Release Tag**: Push a `v0.1.0` tag with finalized binaries/instructions.
3.  **Draft Submission**: Write the one-paragraph project description and assemble all links (Repo, Docs, Demo Video) as required by the submission guidelines.
