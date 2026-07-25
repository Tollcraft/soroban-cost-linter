# Lint Reference

This section provides detailed documentation for all lints supported by `soroban-cost-linter`.

{% hint style="info" %}
See the [Cost Rationale](../cost_rationale.md) page for a full explanation of Soroban's metered resources and why each resource matters.
{% endhint %}

## Storage Operations

| Lint                                                                  | Default Severity | Catches                                    |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`soroban_storage_in_loop`](soroban_storage_in_loop.md)               | `warn`           | Storage reads/writes inside loop bodies    |
| [`storage_write_without_read`](storage_write_without_read.md)         | `warn`           | Storage writes without a corresponding read |
| [`map_insert_in_loop`](map_insert_in_loop.md)                         | `warn`           | `Map::insert` calls inside loops           |

## CPU/Compute

| Lint                                                                  | Default Severity | Catches                                    |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`unnecessary_host_function_call`](unnecessary_host_function_call.md) | `warn`           | Redundant host function calls inside loops |
| [`host_in_loop`](host_in_loop.md)                                     | `warn`           | Host object usage inside loop bodies       |

## Memory

| Lint                                                                  | Default Severity | Catches                                    |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`redundant_env_clone`](redundant_env_clone.md)                       | `warn`           | Unnecessary `.clone()` calls on `Env`      |
| [`inefficient_bytes_concat`](inefficient_bytes_concat.md)             | `warn`           | Repeated `Bytes` concatenation in loops with unnecessary allocations |
| [`bytes_append_in_loop`](bytes_append_in_loop.md)                   | `warn`           | Growth-method calls on `Bytes`/`Vec`/`Map` inside loops |

## Symbol Operations

| Lint                                                                  | Default Severity | Catches                                    |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`symbol_new_for_short_literal`](symbol_new_for_short_literal.md)     | `warn`           | `Symbol::new` with short literal arguments |

## Lint inventory schema

The CLI can emit a versioned inventory of all registered lints via `cargo cost-lint --list-lints --format json`. The payload contains:

- `version`: inventory schema version (`1.0`)
- `schema`: the schema documentation URL
- `lints`: an array of entries containing `name`, `default_level`, `description`, `category`, and `documentation_url`

{% hint style="info" %}
Severities can be adjusted per-workspace via `budget.toml` — see the [Integration Guide](../integration.md).
{% endhint %}

<!-- ? -->