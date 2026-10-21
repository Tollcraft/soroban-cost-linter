# Pending UI fixtures

Every file here is a UI fixture (`<lint>.rs` plus its expected `<lint>.stderr`)
for a lint that **is not registered in `register_lints` today**. They were
found in `ui/` while repairing the source corruption that left `main` unable
to compile.

They cannot stay in `ui/`. `dylint_testing::ui_test` compiles each fixture and
diffs the real diagnostics against the `.stderr`; with no lint registered the
compiler emits `unknown lint: <name>` instead of the expected warning, so every
one of these fails. `ui_fixtures_match_registered_lints_and_have_no_unknown_lint_warnings`
enforces exactly that invariant — a fixture in `ui/` must name a registered
lint. Deleting them would have thrown away the specification each one carries,
so they are parked here instead.

**Nothing in this directory is compiled or run by the test suite.** Moving a
pair back into `ui/` is the last step of implementing its lint, not the first.

## Restoring one

Each of these had a working implementation in git history that a bad merge
dropped. Start from that commit — the pass body is there, though it will need
adapting to the pinned `nightly-2026-04-16` rustc API (the usual drift:
`TyCtxt::hir()` -> `parent_hir_node`, `Node::Local` -> `Node::LetStmt`,
`ExprKind::ForLoop` -> `clippy_utils::higher::ForLoop`, `LateContext::span_lint`
-> `span_lint_and_help`).

| lint | implementation to restore from |
| --- | --- |
| `blind_storage_write` | `8659a92` |
| `bytes_slice_copy_in_loop` | `04d5b1b` |
| `excessive_vec_capacity` | `833d958` |
| `nested_loop_storage_access` | `49101f5` |
| `redundant_val_conversion` | `1aa0baf` |
| `std_collection_in_contract` | `a77f614` |
| `temporary_storage_for_persistent_data` | `ed0d988` |

The remaining five never had an implementation in this repository at all — the
fixture is the whole specification, and the pass has to be written:

- `instance_storage_write_in_loop`
- `persistent_storage_for_ephemeral_data`
- `redundant_address_clone`
- `storage_read_never_written`
- `vec_index_in_loop`

## `u128_where_u64_suffices` is a special case

`U128_WHERE_U64_SUFFICES` **is** declared and listed in `register_lints`, so it
passes the fixture/registration guard — but no pass has ever implemented it in
this repository's history, so it fires nothing. Its `.stderr` was hand-written
rather than generated (it still carries compiletest's pre-`-Zui-testing` line
numbering, `5 |` instead of `LL |`, and reports at `error` level where the lint
is declared `Warn`), which is why it never matched real output. The fixture is
parked here until a pass exists; the lint declaration itself is left in place so
`#[allow(u128_where_u64_suffices)]` in downstream code keeps compiling.

## Checklist for re-landing a lint

1. Implement the pass and declare its lint.
2. Add the lint to `register_lints` and register the pass.
3. Add its metadata entry so `generate-lint-docs` covers it.
4. `git mv` the `.rs` and `.stderr` pair back into `ui/`.
5. Regenerate the expected output rather than hand-editing it, then
   `cargo test -p soroban_cost_lints`.
