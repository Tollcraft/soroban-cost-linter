# Lint Reference

This section provides detailed documentation for all lints supported by `soroban-cost-linter`.

| Lint                                                                  | Default Severity | Catches                                    | Target Resource |
| --------------------------------------------------------------------- | ---------------- | ------------------------------------------ | --------------- |
| [`soroban_storage_in_loop`](soroban_storage_in_loop.md)               | `warn`           | Storage reads/writes inside loop bodies    | [Storage](../cost_rationale.md#per-lint-resource-summary) |
| [`redundant_env_clone`](redundant_env_clone.md)                       | `warn`           | Unnecessary `.clone()` calls on `Env`      | [CPU](../cost_rationale.md#per-lint-resource-summary) |
| [`unnecessary_host_function_call`](unnecessary_host_function_call.md) | `warn`           | Redundant host function calls inside loops | [CPU](../cost_rationale.md#per-lint-resource-summary) |

{% hint style="info" %}
See the [Cost Rationale](../cost_rationale.md) page for a full explanation of Soroban's metered resources and why each resource matters.
{% endhint %}

{% hint style="info" %}
Severities can be adjusted per-workspace via `budget.toml` — see the [Integration Guide](../integration.md).
{% endhint %}
