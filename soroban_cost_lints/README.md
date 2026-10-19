# soroban_cost_lints

The lint crate for `soroban-cost-linter`. Compiled as a dynamic library (`cdylib`) and loaded by [Dylint](https://github.com/trailofbits/dylint) at runtime. See [`DEVELOPING_LINTS.md`](../DEVELOPING_LINTS.md) for how the library is built and loaded.

## Registered Lints

This crate registers **40 lints** via `rustc_session::declare_lint!` in [`src/lib.rs`](src/lib.rs). See [`docs/lints/README.md`](../docs/lints/README.md) for the full generated reference with default severity, category, and documentation links.
