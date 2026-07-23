# soroban_cost_lints

The lint crate for `soroban-cost-linter`. Compiled as a dynamic library and loaded by Dylint at runtime.

## Registered Lints

| Lint | Description |
|------|-------------|
| [`soroban_storage_in_loop`](../docs/lints/soroban_storage_in_loop.md) | Flags storage read/write operations inside loop bodies, suggesting memory aggregation instead. |
| [`redundant_env_clone`](../docs/lints/redundant_env_clone.md) | Detects unnecessary `.clone()` calls on the Soroban `Env` object. |
| [`unnecessary_host_function_call`](../docs/lints/unnecessary_host_function_call.md) | Identifies redundant calls to host functions (like `Ledger` sequences) inside loops that should be called once and cached. |

See the [lint reference](../docs/lints/README.md) for full documentation.
