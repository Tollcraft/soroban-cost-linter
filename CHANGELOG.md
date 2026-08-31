# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to Semantic Versioning.

## [Unreleased]

### Added

- New lint `symbol_new_for_short_literal` detecting `Symbol::new(&env, "literal")` calls where the literal is a valid short symbol (≤ 9 chars, alphanumeric + underscore) and suggesting the `symbol_short!` macro for compile-time creation.
- New lint `bytes_append_in_loop` flagging growth-method calls (`append`, `push_back`, `insert`, `extend_from_array`) on Soroban SDK containers (`Bytes`, `Vec`, `Map`) inside loop bodies.
- New lint `require_auth_in_loop` detecting `Address::require_auth` and `Address::require_auth_for_args` calls inside loop bodies (`for`, `while`, `loop`) and suggesting that distinct addresses be authorized once before the loop.
- New lint `redundant_require_auth` detecting duplicate `Address::require_auth` / `Address::require_auth_for_args` calls on the same address within a single function body without an intervening cross-contract call. Authorization context resets across `env.invoke_contract` and `env.try_invoke_contract` boundaries.
- New lint `storage_write_without_read` detecting storage `.set()` calls where the same key is never subsequently read, flagging wasteful storage writes that drive up Soroban fees.
- New lint `inefficient_bytes_concat` detecting repeated `Bytes` concatenation using `+` inside loops, which creates unnecessary per-iteration allocations.
- New lint `map_insert_in_loop` detecting `Map::insert()` calls inside loop bodies.
- New lint `signature_verification_in_loop` detecting `env.crypto().ed25519_verify()`/`secp256k1_recover()`/`secp256r1_verify()` calls inside loop bodies, suggesting batch/aggregate signature verification or a bulk callee entrypoint instead.
- New lint `instance_storage_for_unbounded_data` detecting `env.storage().instance().set(...)` calls where the written value is an unbounded `Vec`/`Map`/`Bytes`, since instance storage is re-read and rewritten as a single blob on every contract invocation regardless of which call touches it.
- New lint `formatted_panic_payload` detecting `format!(...)`, `panic!(...)` with formatting arguments, and `.expect(&format!(...))`, all of which pull `core::fmt` string-formatting machinery into a `#![no_std]` contract; the diagnostic points at `panic_with_error!` with a `#[contracterror]` enum as the cheap alternative. Skips `#[cfg(test)]` code.
- Integration test `documented_formats_are_accepted` asserting every `--format` value documented in the README (`text`, `json`, `sarif`, `github`) is accepted by the CLI, with a negative control to prevent future docs/CLI drift.
- New lint `loop_invariant_storage_access` detecting storage operations inside loops whose operands are provably loop-invariant, so the access can be hoisted out of the loop.
- New lint `soroban_redundant_storage_read` detecting multiple sequential reads of the same storage key without an intervening modification.
- New lint `soroban_inefficient_bytes_concat` detecting inefficient `Bytes` concatenation inside loop bodies.
- New lint `host_in_loop` detecting use of a `Host` object inside a loop.
- New lint `contract_call_in_loop` detecting cross-contract invocation inside loop bodies, where each call pays VM instantiation and dispatch overhead.
- New lint `unbounded_input_loop` detecting loop bounds derived from untrusted input with a storage write in the body, flagging caller-controlled storage write amplification.
- New lint `storage_key_construction_in_loop` detecting storage keys constructed inside a loop body where they could be hoisted.
- New lint `vec_where_slice_could_be_used` detecting `soroban_sdk::Vec` passed by value where a native Rust slice would suffice.
- New lint `extend_ttl_in_loop` detecting `extend_ttl` calls inside loop bodies, which pay a metered host call and rent per iteration.
- New lint `linear_scan_in_loop` detecting linear scans over collections inside loops, which turn O(n) work into O(n²).
- New lint `persistent_read_without_ttl_extension` detecting reads of persistent storage without a TTL extension, avoiding the archival cost cliff.
- New lint `unnecessary_string_to_bytes` detecting unnecessary `String` to `Bytes` conversions.
- New lint `unbounded_recursion` detecting direct and mutual recursion whose depth is driven by caller-supplied input (e.g. recursion over a caller-supplied `Vec`/`&[T]` length), reporting the full call cycle (`process -> process_child -> process`).
- New lint `cross_contract_result_discarded` detecting `Env::invoke_contract` calls whose non-unit return value is discarded (bound to `_` or dropped as a bare statement), since a cross-contract invocation pays for a full host dispatch, metered execution, and the return value's conversion back across the boundary.
- New lint `storage_read_never_written` detecting storage reads of a key that is never written by a statically-known `set`/`has` anywhere else in the crate. It accumulates reads and writes across the whole crate and reports only at the end of the crate, firing at the read site. Defaults to `warn` (not `deny`) because it is heuristic: the write may live in another contract (cross-contract state sharing), or the key may be constructed dynamically. Dynamic keys neither fire nor suppress findings about unrelated static keys.

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
  `docs/integration.md`. A `budget.toml` in the directory the tool is run from
  is picked up automatically; `--config <PATH>` points it at a config
  elsewhere. The tool does not walk up to a workspace-root `budget.toml`; when
  no config is found, every lint runs at its declared default level.
- Split the `soroban_storage_in_loop` lint page into a `Writes (set)` and
  `Reads (get, has)` section, each with its own suggested-fix hint that lines
  up with the diagnostic help text.
- Added `docs/lint_categories.md` documenting the five `LintCategory` values
  (`StorageOperations`, `Compute`, `Memory`, `EntryLifecycle`,
  `SymbolOperations`), the metered Soroban resource each maps to, and how to
  pick a category for a new lint. The contributing and lint-authoring guides
  now link to it instead of carrying an incomplete inline list.

## [0.1.1]

### Changed

- Updated the workspace crate versions to `0.1.1` in preparation for the release.

## [0.1.0]

### Added

- Three built-in Soroban cost lints.
- `cargo-cost-lint` CLI wrapper.
- Support for configuring lint levels using `budget.toml`.
