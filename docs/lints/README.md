# Lint Reference

This section provides detailed documentation for all lints supported by `soroban-cost-linter`.

{% hint style="info" %}
See the [Cost Rationale](../cost_rationale.md) page for a full explanation of Soroban's metered resources and why each resource matters.
{% endhint %}

## Confidence / Impact Classification

Lints are classified per the [MVP roadmap](../roadmap_mvp.md#3-false-positive-mitigation-strategy):

| Classification | Default Level | Meaning |
|---|---|---|
| **High Confidence, High Impact** | `deny` | Pattern is unambiguous and always expensive. Fails CI by default. |
| **Medium Impact / Context-Dependent** | `warn` | Pattern may be acceptable in small or bounded contexts. Does not block CI by default. |

## Storage Operations

| Lint                                                                  | Default Severity | Catches                                    |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`soroban_storage_in_loop`](soroban_storage_in_loop.md)               | `warn`           | Storage reads/writes inside loop bodies    |
| [`storage_write_without_read`](storage_write_without_read.md)         | `warn`           | Storage writes without a corresponding read |
| [`map_insert_in_loop`](map_insert_in_loop.md)                         | `warn`           | `Map::insert` calls inside loops           |

## CPU/Compute

| Lint | Default Severity | Confidence / Impact | Catches |
|---|---|---|---|
| [`unnecessary_host_function_call`](unnecessary_host_function_call.md) | `warn` | Medium Impact, Context-Dependent | `Ledger`, `Crypto`, `Prng`, `Events`, `Deployer` and `Env::current_contract_address` calls repeated inside loops with unchanged inputs |

## Memory

| Lint                                                                  | Default Severity | Catches                                    |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`redundant_env_clone`](redundant_env_clone.md)                       | `warn`           | Unnecessary `.clone()` calls on `Env`      |
| [`inefficient_bytes_concat`](inefficient_bytes_concat.md)             | `warn`           | Repeated `Bytes` concatenation in loops with unnecessary allocations |
| [`bytes_append_in_loop`](bytes_append_in_loop.md)                   | `warn`           | Growth-method calls on `Bytes`/`Vec`/`Map` inside loops |

## Symbol Operations

| Lint | Default Severity | Confidence / Impact | Catches |
|---|---|---|---|
| [`symbol_new_for_short_literal`](symbol_new_for_short_literal.md) | `warn` | Medium Impact | `Symbol::new` with short literal arguments |

{% hint style="info" %}
Severities can be adjusted per-workspace via `budget.toml` — see the [Integration Guide](../integration.md).
{% endhint %}

<!-- ? -->
