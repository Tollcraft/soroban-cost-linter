# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to Semantic Versioning.

## [Unreleased]

### Added

- New lint `symbol_new_for_short_literal` detecting `Symbol::new(&env, "literal")` calls where the literal is a valid short symbol (≤ 9 chars, alphanumeric + underscore) and suggesting the `symbol_short!` macro for compile-time creation.
- New lint `blind_storage_write` detecting storage `.set()` calls on `Instance`, `Persistent`, or `Temporary` that have no preceding read (`.get()`, `.try_get()`, `.has()`, `.remove()`, or `.update()`) on the same key anywhere in the function body. This catches silent overwrites and accidental key collisions before they reach the network.

## [0.1.1]

### Changed

- Updated the workspace crate versions to `0.1.1` in preparation for the release.

## [0.1.0]

### Added

- Three built-in Soroban cost lints.
- `cargo-cost-lint` CLI wrapper.
- Support for configuring lint levels using `budget.toml`.
