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

## Lints Covered

| Lint | Test function | What is measured |
| --- | --- | --- |
| `symbol_new_for_short_literal` | `bench_symbol_new_vs_short` | `Symbol::new` vs `symbol_short!` |
| `redundant_env_clone` | `bench_env_clone_vs_reuse` | `env.clone()` vs `&env` |
| `unnecessary_host_function_call` | `bench_host_fn_inside_vs_outside_loop` | Host fn in loop vs hoisted |
| `bytes_append_in_loop` | `bench_bytes_append_in_loop_vs_batch` | `Bytes::append` in loop vs native Vec batch |
| `soroban_storage_in_loop` | `bench_storage_in_loop_vs_batch` | Storage writes in loop vs accumulate + one write |

## Reproducibility

All measurements use `Env::default()` (a local test-only environment, not a network simulation). The numbers are **directional** — they show relative savings between the bad and good patterns — but are subject to the local-vs-network gap described in the [Cost Rationale](https://tollcraft.gitbook.io/docs/soroban-cost-linter/concepts/cost-rationale#the-local-vs-network-gap).

For network-accurate measurements, use the sibling project [`soroban-budget-assert`](https://github.com/Tollcraft/soroban-budget-assert).

## Requirements

- Rust stable (edition 2021)
- `soroban-sdk = "26"`
