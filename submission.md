# Drips Wave 7 — Submission

**Project Name:** `soroban-cost-linter`
**Track/Bounty:** Stellar Wave Builder — Master System Prompt

## 1-Paragraph Description
`soroban-cost-linter` is a specialized static analysis tool for Soroban smart contracts, built to identify structurally expensive anti-patterns before compilation. By leveraging `dylint` to hook directly into the Rust compiler's High-Level Intermediate Representation (HIR), the tool reliably flags budget-draining practices—such as storage operations or redundant host function calls inside loops—that basic regex linters would miss. Designed to fit seamlessly into GitHub Actions CI/CD pipelines via the `cargo cost-lint` CLI wrapper, it acts as the preventative Stage 1 shield within the Tollcraft cost-awareness ecosystem.

## Project Links
*   **Repository:** [https://github.com/Tollcraft/soroban-cost-linter](https://github.com/Tollcraft/soroban-cost-linter)
*   **Release Tag:** [v0.1.0](https://github.com/Tollcraft/soroban-cost-linter/releases/tag/v0.1.0)
*   **Documentation Site:** [Docs (mdBook)](https://tollcraft.github.io/soroban-cost-linter/) *(Placeholder URL)*
*   **Demo Video:** [YouTube / Loom Link here] *(Placeholder URL)*
