# Lint Reference

This section provides detailed documentation for all lints supported by `soroban-cost-linter`.

# Lint Reference

This section provides detailed documentation for all lints supported by `soroban-cost-linter`.

## Storage Operations

| Lint                                                                  | Default Severity | Catches                                    | Target Resource |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ | --------------- |
| [`soroban_storage_in_loop`](soroban_storage_in_loop.md)               | `warn`           | Storage reads/writes inside loop bodies    | [Storage](../cost_rationale.md#per-lint-resource-summary) |

## CPU/Compute

<<<<<<< HEAD
| Lint                                                                  | Default Severity | Catches                                    | Target Resource |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ | --------------- |
| [`unnecessary_host_function_call`](unnecessary_host_function_call.md) | `warn`           | Redundant host function calls inside loops | [CPU](../cost_rationale.md#per-lint-resource-summary) |
| [`event_in_loop`](event_in_loop.md)                                   | `warn`           | Event emissions (`env.events().publish`) inside loops | [Compute](../cost_rationale.md#per-lint-resource-summary) |
=======
| Lint                                                                  | Default Severity | Catches                                    |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`unnecessary_host_function_call`](unnecessary_host_function_call.md) | `warn`           | `Ledger`, `Crypto`, `Prng`, `Events`, `Deployer` and `Env::current_contract_address` calls repeated inside loops with unchanged inputs |
>>>>>>> f3107b6 (feat: extend unnecessary_host_function_call to all host object types (#185))

## Memory

| Lint                                                                  | Default Severity | Catches                                    | Target Resource |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ | --------------- |
| [`redundant_env_clone`](redundant_env_clone.md)                       | `warn`           | Unnecessary `.clone()` calls on `Env`      | [CPU](../cost_rationale.md#per-lint-resource-summary) |

## Symbol Operations

| Lint                                                                  | Default Severity | Catches                                    | Target Resource |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ | --------------- |
| [`symbol_new_for_short_literal`](symbol_new_for_short_literal.md)     | `warn`           | `Symbol::new` with short literal arguments | [CPU](../cost_rationale.md#per-lint-resource-summary) |

{% hint style="info" %}
See the [Cost Rationale](../cost_rationale.md) page for a full explanation of Soroban's metered resources and why each resource matters.
{% endhint %}

<<<<<<< HEAD
=======
## Storage Operations

| Lint                                                                  | Default Severity | Catches                                    |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`soroban_storage_in_loop`](soroban_storage_in_loop.md)               | `warn`           | Storage reads/writes inside loop bodies    |
| [`storage_write_without_read`](storage_write_without_read.md)         | `warn`           | Storage writes without a corresponding read |
| [`map_insert_in_loop`](map_insert_in_loop.md)                         | `warn`           | `Map::insert` calls inside loops           |

## CPU/Compute

| Lint                                                                  | Default Severity | Catches                                    |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`unnecessary_host_function_call`](unnecessary_host_function_call.md) | `warn`           | `Ledger`, `Crypto`, `Prng`, `Events`, `Deployer` and `Env::current_contract_address` calls repeated inside loops with unchanged inputs |
| [`signature_verification_in_loop`](signature_verification_in_loop.md) | `warn`           | `ed25519_verify`/`secp256k1_recover`/`secp256r1_verify` calls inside loops |

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

>>>>>>> 02403c6 (Implement bytes_append_in_loop lint (#226))
{% hint style="info" %}
Severities can be adjusted per-workspace via `budget.toml` — see the [Integration Guide](../integration.md).
{% endhint %}

<!-- ? -->