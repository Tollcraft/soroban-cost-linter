# Lint Catalog

This document provides a concise reference for all lints supported by **soroban-cost-linter**. Each entry includes the lint name, its default severity, a brief description, and a link to the full documentation.

| Lint | Default Severity | Description | Docs |
|------|------------------|-------------|------|
| `soroban_storage_in_loop` | warn | Detects storage reads/writes inside loop bodies. | [Link](lints/soroban_storage_in_loop.md) |
| `storage_write_without_read` | warn | Flags storage writes without a corresponding read. | [Link](lints/storage_write_without_read.md) |
| `map_insert_in_loop` | warn | Identifies `Map::insert` calls inside loops. | [Link](lints/map_insert_in_loop.md) |
| `unnecessary_host_function_call` | warn | Flags repeated host function calls inside loops when inputs unchanged. | [Link](lints/unnecessary_host_function_call.md) |
| `signature_verification_in_loop` | warn | Detects cryptographic verification calls inside loops. | [Link](lints/signature_verification_in_loop.md) |
| `redundant_env_clone` | warn | Unnecessary `.clone()` calls on the `Env` object. | [Link](lints/redundant_env_clone.md) |
| `inefficient_bytes_concat` | warn | Repeated `Bytes` concatenation in loops causing allocations. | [Link](lints/inefficient_bytes_concat.md) |
| `bytes_append_in_loop` | warn | Growth-method calls on `Bytes`/`Vec`/`Map` inside loops. | [Link](lints/bytes_append_in_loop.md) |
| `excessive_vec_capacity` | warn | Large, hard‑coded capacity in `Vec::with_capacity`/`.reserve`. | [Link](lints/excessive_vec_capacity.md) |
| `vec_where_slice_could_be_used` | warn | Uses `Vec` by value where a slice would suffice. | [Link](lints/vec_where_slice_could_be_used.md) |
| `symbol_new_for_short_literal` | warn | `Symbol::new` with short literal arguments. | [Link](lints/symbol_new_for_short_literal.md) |
| `storage_key_construction_in_loop` | warn | Loop‑invariant `Symbol::new` key construction inside loops. | [Link](lints/storage_key_construction_in_loop.md) |
| `host_in_loop` | warn | Detects host functions called inside loops unnecessarily. | [Link](lints/host_in_loop.md) |
| `collection_len_in_loop_condition` | warn | Recalculates collection length in loop condition each iteration. | [Link](lints/collection_len_in_loop_condition.md) |
| `discarded_storage_read` | warn | Reads from storage whose result is never used. | [Link](lints/discarded_storage_read.md) |
| `redundant_val_conversion` | warn | Redundant conversions of values that are already in the correct type. | [Link](lints/redundant_val_conversion.md) |
| `unnecessary_string_to_bytes` | warn | Unnecessary conversion from `String` to `Bytes`. | [Link](lints/unnecessary_string_to_bytes.md) |

*Severities can be overridden via `budget.toml`.*
