# Cost Benchmarks

Cost measurement harness for [soroban-cost-linter](https://github.com/Tollcraft/soroban-cost-linter) lint reference pages.

Each test reproduces a specific anti-pattern flagged by one of the linter's rules and its suggested fix on a minimal example, then prints the `env.cost_estimate().budget()` deltas so the figures can be copied into the lint documentation.

## Usage

```bash
cd cost_benchmarks
cargo test -- --nocapture
```

The output is formatted as a table. Each line is
`lint_name | bad|good | scenario | cpu: N  mem: M`, printed with fixed column
widths of 25 (lint), 4 (variant) and 34 (scenario):

```
symbol_new_for_short_literal | bad  | Symbol::new in loop                | cpu: <CPU>  mem: <MEM>
symbol_new_for_short_literal | good | symbol_short! in loop              | cpu: <CPU>  mem: <MEM>
redundant_env_clone       | bad  | env.clone() in loop                | cpu: <CPU>  mem: <MEM>
redundant_env_clone       | good | &env in loop                       | cpu: <CPU>  mem: <MEM>
unnecessary_host_fn_call  | bad  | ledger().sequence() in loop        | cpu: <CPU>  mem: <MEM>
unnecessary_host_fn_call  | good | hoisted + reuse in loop            | cpu: <CPU>  mem: <MEM>
...
```

`cpu` is the CPU-instruction-count delta and `mem` is the memory-byte delta
reported by the host budget. The `<CPU>` and `<MEM>` figures above are
**placeholders** and must be replaced with the lines from a real run; this
document was not regenerated against a working toolchain. A real run prints
the numeric counts, and is expected to use the repository's pinned toolchain
(Rust `nightly-2026-04-16`, from `rust-toolchain`) and `soroban-sdk = "26"`
(from `cost_benchmarks/Cargo.toml`).

You can read this output by comparing the `bad` (anti-pattern) and `good` (recommended fix) measurements. The difference between the two is the exact cost savings for that particular lint pattern in a local environment. A lint pattern that costs 0 additional resources might be a correctness lint (e.g. `storage_write_without_read`), meaning that adopting the fix prevents bugs without any significant performance hit.

## Lints Covered

| Lint | Test function | What is measured |
| --- | --- | --- |
| `symbol_new_for_short_literal` | `bench_symbol_new_vs_short` | `Symbol::new` in loop vs `symbol_short!` in loop |
| `redundant_env_clone` | `bench_env_clone_vs_reuse` | `env.clone()` in loop vs `&env` in loop |
| `unnecessary_host_function_call` | `bench_host_fn_inside_vs_outside_loop` | `ledger().sequence()` in loop vs hoisted and reused |
| `bytes_append_in_loop` | `bench_bytes_append_in_loop_vs_batch` | `Bytes::append` in loop vs native `Vec` + `from_slice` |
| `soroban_storage_in_loop` | `bench_storage_in_loop_vs_batch` | `instance().set()` in loop vs accumulate + one write |
| `storage_write_without_read` | `bench_blind_storage_write` | blind `.set()` vs `.get()` then `.set()` |
| `discarded_storage_read` | `bench_discarded_storage_read` | discarded `.get()` vs `.has()` |
| `soroban_redundant_storage_read` | `bench_redundant_storage_read` | `get()` twice vs `get()` once and reuse |
| `instance_storage_for_unbounded_data` | `bench_instance_storage_unbounded_data` | `Vec` in instance storage vs entry in persistent storage |
| `loop_invariant_storage_access` | `bench_loop_invariant_storage_access` | `get()` inside loop vs `get()` outside loop |
| `storage_key_construction_in_loop` | `bench_storage_key_construction_in_loop` | build key in loop vs hoisted key |

## Unmeasured Lints

The following registered lints are **unmeasured** in this benchmark harness:
- `contract_call_in_loop`
- `crypto_hash_of_constant`
- `duplicate_storage_key_construction`
- `extend_ttl_in_loop`
- `float_arithmetic_in_contract`
- `formatted_panic_payload`
- `host_in_loop`
- `inefficient_bytes_concat`
- `large_constant_array`
- `ledger_context_read_in_loop`
- `linear_scan_in_loop`
- `map_insert_in_loop`
- `option_wrapping_in_storage`
- `persistent_read_without_ttl_extension`
- `redundant_require_auth`
- `require_auth_in_loop`
- `signature_verification_in_loop`
- `soroban_inefficient_bytes_concat`
- `string_concat_in_loop`
- `symbol_key_boundary`
- `symbol_key_enum_storage`
- `symbol_key_event_topics`
- `u128_where_u64_suffices`
- `unbounded_input_loop`
- `unbounded_recursion`
- `unnecessary_host_function_call_legacy`
- `unnecessary_string_to_bytes`
- `unwrap_on_storage_get`
- `vec_where_slice_could_be_used`

*(If you are measuring a new group of lints, please add them to the "Covered" table and remove them from the "Unmeasured" list!)*

## Reproducibility

All measurements use `Env::default()` (a local test-only environment, not a network simulation). The numbers are **directional** — they show relative savings between the bad and good patterns — but are subject to the local-vs-network gap described in the [Cost Rationale](https://tollcraft.gitbook.io/docs/soroban-cost-linter/concepts/cost-rationale#the-local-vs-network-gap).

For network-accurate measurements, use the sibling project [`soroban-budget-assert`](https://github.com/Tollcraft/soroban-budget-assert).

## Requirements

- Rust stable (edition 2021)
- `soroban-sdk = "26"`
