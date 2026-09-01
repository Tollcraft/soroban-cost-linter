# Writing a Custom Lint

The authoritative, up-to-date guide for adding a new lint to `soroban-cost-linter` is **[`DEVELOPING_LINTS.md`](../DEVELOPING_LINTS.md)** at the repository root (also at [`DEVELOPING_LINTS.md` on GitHub](https://github.com/Tollcraft/soroban-cost-linter/blob/main/DEVELOPING_LINTS.md)).

Follow that guide. It contains everything you need: where lint declarations and registration live (`soroban_cost_lints/src/lib.rs`, with the three coordinated registration edits), where UI fixtures go (`soroban_cost_lints/ui/`), how to use `clippy_utils`, an end-to-end walkthrough of a real lint, and the exact commands CI enforces.

The older content on this page is intentionally removed because it described a `soroban_cost_lints/src/lints/` directory, a `tests/ui/` fixture directory, and a `cargo test --test ui_<name>` target that do not exist in this repository — none of it would work.
