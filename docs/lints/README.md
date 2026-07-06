# 🔍 Lint Reference

This section provides detailed documentation for all lints supported by `soroban-cost-linter`.

| Lint                                                                | Default Severity | Catches                                    |
| ------------------------------------------------------------------- | ---------------- | ------------------------------------------ |
| [`soroban_storage_in_loop`](soroban_storage_in_loop.md)             | `warn`           | Storage reads/writes inside loop bodies    |
| [`redundant_env_clone`](redundant_env_clone.md)                     | `warn`           | Unnecessary `.clone()` calls on `Env`      |
| [`unnecessary_host_function_call`](unnecessary_host_function_call.md) | `warn`         | Redundant host function calls inside loops |

{% hint style="info" %}
Severities can be adjusted per-workspace via `budget.toml` — see the [Integration Guide](../integration.md).
{% endhint %}
