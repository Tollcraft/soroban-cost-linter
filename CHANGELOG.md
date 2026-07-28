# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to Semantic Versioning.

## [Unreleased]

### Added

- New lint `symbol_new_for_short_literal` detecting `Symbol::new(&env, "literal")` calls where the literal is a valid short symbol (≤ 9 chars, alphanumeric + underscore) and suggesting the `symbol_short!` macro for compile-time creation.
- New lint `storage_write_without_read` detecting storage `.set()` calls where the same key is never subsequently read, flagging wasteful storage writes that drive up Soroban fees.
- New lint `inefficient_bytes_concat` detecting repeated `Bytes` concatenation using `+` inside loops, which creates unnecessary per-iteration allocations.
- New lint `map_insert_in_loop` detecting `Map::insert()` calls inside loop bodies.
- `--fix` flag for `cargo-cost-lint` to automatically apply machine-applicable lint suggestions in-place.
- `docs/windows_setup.md` covering WSL2 (recommended) and native-PowerShell install, PATH setup, prerequisites (Visual Studio Build Tools, MSVC toolchain, long path support), and common Windows-specific issues. Linked from `CONTRIBUTING.md`, `README.md`, and the `docs/SUMMARY.md` TOC.

### Fixed

- Confirmed that `src/module_17.rs` does not exist and the codebase contains no bitwise manipulation logic; issue #207 is invalid.

### Changed

- `unnecessary_host_function_call` now covers every host accessor reachable from
  `Env` — `crypto()`, `prng()`, `events()`, `deployer()` and
  `current_contract_address()` alongside `ledger()` — and no longer reports
  calls whose receiver or arguments change from iteration to iteration.

## [0.1.1]

### Changed

- Updated the workspace crate versions to `0.1.1` in preparation for the release.

## [0.1.0]

### Added

- Three built-in Soroban cost lints.
- `cargo-cost-lint` CLI wrapper.
- Support for configuring lint levels using `budget.toml`.
