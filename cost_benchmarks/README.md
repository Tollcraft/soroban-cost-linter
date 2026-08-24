# Cost Benchmarks

Cost measurement harness for [soroban-cost-linter](https://github.com/Tollcraft/soroban-cost-linter) lint reference pages.

Each test reproduces a specific anti-pattern flagged by one of the linter's rules and its suggested fix on a minimal example, then prints the `env.budget()` deltas so the figures can be copied into the lint documentation.

## Usage

```bash
cd cost_benchmarks
cargo test -- --nocapture
```

The output is formatted as a table:

```
symbol_new_for_short_literal | bad  | Symbol::new x100   | cpu: 12345
symbol_new_for_short_literal | good | symbol_short! x100 | cpu: 0
...
```
You can read this output by comparing the `bad` (anti-pattern) and `good` (recommended fix) measurements. The difference between the two is the exact cost savings for that particular lint pattern in a local environment. A lint pattern that costs 0 additional resources might be a correctness lint (e.g. `storage_write_without_read`), meaning that adopting the fix prevents bugs without any significant performance hit.

## Lints Covered

| Lint | Test function | What is measured |
| --- | --- | --- |
| `symbol_new_for_short_literal` | `bench_symbol_new_vs_short` | `Symbol::new` vs `symbol_short!` |
| `redundant_env_clone` | `bench_env_clone_vs_reuse` | `env.clone()` vs `&env` |
| `unnecessary_host_function_call` | `bench_host_fn_inside_vs_outside_loop` | Host fn in loop vs hoisted |
| `bytes_append_in_loop` | `bench_bytes_append_in_loop_vs_batch` | `Bytes::append` in loop vs native Vec batch |
| `soroban_storage_in_loop` | `bench_storage_in_loop_vs_batch` | Storage writes in loop vs accumulate + one write |
| `blind_storage_write` | `bench_blind_storage_write` | blind `.set()` vs `.get()` then `.set()` |
| `storage_write_without_read` | `bench_blind_storage_write` | same pattern as `blind_storage_write` |
| `discarded_storage_read` | `bench_discarded_storage_read` | discarded `.get()` vs `.has()` |
| `soroban_redundant_storage_read` | `bench_redundant_storage_read` | `get()` twice vs `get()` once and reuse |
| `instance_storage_for_unbounded_data` | `bench_instance_storage_unbounded_data` | Vec in instance storage vs entry in persistent storage |
| `loop_invariant_storage_access` | `bench_loop_invariant_storage_access` | `get()` inside loop vs `get()` outside loop |
| `storage_key_construction_in_loop` | `bench_storage_key_construction_in_loop` | build key in loop vs build key outside loop |

## Unmeasured Lints

The following lints remain **unmeasured** in this benchmark harness:
- `collection_len_in_loop_condition`
- `contract_call_in_loop`
- `excessive_vec_capacity`
- `extend_ttl_in_loop`
- `formatted_panic_payload`
- `host_in_loop`
- `inefficient_bytes_concat`
- `linear_scan_in_loop`
- `map_insert_in_loop`
- `persistent_read_without_ttl_extension`
- `redundant_val_conversion`
- `require_auth_in_loop`
- `signature_verification_in_loop`
- `unbounded_input_loop`
- `unnecessary_string_to_bytes`
- `vec_where_slice_could_be_used`

*(If you are measuring a new group of lints, please add them to the "Covered" table and remove them from the "Unmeasured" list!)*

## Reproducibility

All measurements use `Env::default()` (a local test-only environment, not a network simulation). The numbers are **directional** — they show relative savings between the bad and good patterns — but are subject to the local-vs-network gap described in the [Cost Rationale](https://tollcraft.gitbook.io/docs/soroban-cost-linter/concepts/cost-rationale#the-local-vs-network-gap).

For network-accurate measurements, use the sibling project [`soroban-budget-assert`](https://github.com/Tollcraft/soroban-budget-assert).

## Requirements

- Rust stable (edition 2021)
- `soroban-sdk = "26"`
