# Project Log: `soroban-cost-linter`

This log tracks the progress of the `soroban-cost-linter` project.

## Log Entries

### [2026-07-02] Initialization & Planning
- **Goal**: Establish roadmap and select first lint for the MVP.
- **Action**: Created [roadmap_mvp.md](file:///home/dell/soroban-cost-linter/roadmap_mvp.md) outlining tooling (Dylint), the first lint pattern (`soroban_storage_in_loop`), false-positive mitigation, and CLI integration.
- **Decision**: Approved proceeding with `soroban_storage_in_loop` as Milestone 1.
- **Action**: Initialized [project_log.md](file:///home/dell/soroban-cost-linter/project_log.md) to track all future updates.

### [2026-07-02] Toolchain Setup
- **Goal**: Configure toolchain and install Dylint components for compiler-level linting.
- **Action**: Installed Rust nightly toolchain with `rustc-dev` and `llvm-tools-preview` components.
- **Action**: Completed installation of `cargo-dylint` and `dylint-link`.
- **Status**: Toolchain and Dylint driver setup complete.

### [2026-07-03] Milestone 1: First Lint Implementation
- **Goal**: Implement and test the `soroban_storage_in_loop` lint.
- **Action**: Created `soroban_cost_lints` Dylint library.
- **Action**: Implemented AST/HIR analysis in [lib.rs](file:///home/dell/soroban-cost-linter/soroban_cost_lints/src/lib.rs) checking for method calls on `Env::storage` and `soroban_sdk::storage` types inside enclosing loops.
- **Action**: Added comprehensive UI test suite in [main.rs](file:///home/dell/soroban-cost-linter/soroban_cost_lints/ui/main.rs) and verified using Dylint UI test harness.
- **Status**: Milestone 1 complete. Lint triggers on all loop formats (for, while, loop) and supports standard attribute suppression.

