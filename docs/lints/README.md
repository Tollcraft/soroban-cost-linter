# Lint Reference

This section provides detailed documentation for all lints supported by `soroban-cost-linter`.

{% hint style="info" %}
See the [Cost Rationale](../cost_rationale.md) page for a full explanation of Soroban's metered resources and why each resource matters.
{% endhint %}

## Storage Operations

| Lint                                                                  | Default Severity | Catches                                    |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`soroban_storage_in_loop`](soroban_storage_in_loop.md)               | `warn`           | Storage reads/writes inside loop bodies    |

## CPU/Compute

| Lint                                                                  | Default Severity | Catches                                    |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`unnecessary_host_function_call`](unnecessary_host_function_call.md) | `warn`           | `Ledger`, `Crypto`, `Prng`, `Events`, `Deployer` and `Env::current_contract_address` calls repeated inside loops with unchanged inputs |

## Memory

| Lint                                                                  | Default Severity | Catches                                    |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`redundant_env_clone`](redundant_env_clone.md)                       | `warn`           | Unnecessary `.clone()` calls on `Env`      |

## Symbol Operations

| Lint                                                                  | Default Severity | Catches                                    |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`symbol_new_for_short_literal`](symbol_new_for_short_literal.md)     | `warn`           | `Symbol::new` with short literal arguments |

## Entry Lifecycle

| Lint                                                                                          | Default Severity | Catches                                    |
| --------------------------------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`persistent_read_without_ttl_extension`](persistent_read_without_ttl_extension.md)           | `warn`           | Persistent reads without matching TTL extension |

{% hint style="info" %}
Severities can be adjusted per-workspace via `budget.toml` — see the [Integration Guide](../integration.md).
{% endhint %}
