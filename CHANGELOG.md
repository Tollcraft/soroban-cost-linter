# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to Semantic Versioning.

## [Unreleased]

### Added

- New lint `symbol_new_for_short_literal` detecting `Symbol::new(&env, "literal")` calls where the literal is a valid short symbol (≤ 9 chars, alphanumeric + underscore) and suggesting the `symbol_short!` macro for compile-time creation.
- New lint `bytes_append_in_loop` flagging growth-method calls (`append`, `push_back`, `insert`, `extend_from_array`) on Soroban SDK containers (`Bytes`, `Vec`, `Map`) inside loop bodies.

### Fixed

- Confirmed that `src/module_17.rs` does not exist and the codebase contains no bitwise manipulation logic; issue #207 is invalid.
 <!-- grep -R -nE '<<|>>|&|\||\^|!' src -->
### Changed

- `unnecessary_host_function_call` now covers every host accessor reachable from
  `Env` — `crypto()`, `prng()`, `events()`, `deployer()` and
  `current_contract_address()` alongside `ledger()` — and no longer reports
  calls whose receiver or arguments change from iteration to iteration.
- `unnecessary_host_function_call` now also fires for host function calls inside
  iterator closures (`.iter().for_each(...)` and other multi-call closures),
  matching the behaviour inside `for` / `while` / `loop`.
- `soroban_storage_in_loop` differentiates the help text for storage reads
  versus writes inside loops. Reads (`.get` / `.has`) now suggest hoisting a
  loop-invariant read or batching; writes (`.set`) keep the accumulate-and-flush
  advice.
- `soroban_storage_in_loop` now collapses overlapping warnings on a chained
  expression like `env.storage().instance().set(&k, &v)` into a single warning
  keyed on the terminal `.get` / `.has` / `.set`. Intermediate accessor calls
  no longer contribute separate diagnostics.
- Documented the `--config <PATH>` flag of `cargo cost-lint` in `README.md` and
  `docs/integration.md`. The flag is the only supported way today to apply a
  `budget.toml`; with it omitted, no config is loaded and every lint runs at
  its declared default level (`warn`).
- Split the `soroban_storage_in_loop` lint page into a `Writes (set)` and
  `Reads (get, has)` section, each with its own suggested-fix hint that lines
  up with the diagnostic help text.

## [0.1.1]

### Changed

- Updated the workspace crate versions to `0.1.1` in preparation for the release.

## [0.1.0]

### Added

- Three built-in Soroban cost lints.
- `cargo-cost-lint` CLI wrapper.
- Support for configuring lint levels using `budget.toml`.
