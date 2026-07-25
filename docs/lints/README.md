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

| Lint | Default Severity | Confidence / Impact | Catches |
|---|---|---|---|
| [`soroban_storage_in_loop`](soroban_storage_in_loop.md) | `deny` | High Confidence, High Impact | Storage reads/writes inside loop bodies |

## CPU/Compute

| Lint | Default Severity | Confidence / Impact | Catches |
|---|---|---|---|
| [`unnecessary_host_function_call`](unnecessary_host_function_call.md) | `warn` | Medium Impact, Context-Dependent | `Ledger`, `Crypto`, `Prng`, `Events`, `Deployer` and `Env::current_contract_address` calls repeated inside loops with unchanged inputs |

## Memory

| Lint | Default Severity | Confidence / Impact | Catches |
|---|---|---|---|
| [`redundant_env_clone`](redundant_env_clone.md) | `warn` | Medium Impact | Unnecessary `.clone()` calls on `Env` |
| [`bytes_append_in_loop`](bytes_append_in_loop.md) | `warn` | Medium Impact, Context-Dependent | Growth-method calls on `Bytes`/`Vec`/`Map` inside loops |

## Symbol Operations

| Lint | Default Severity | Confidence / Impact | Catches |
|---|---|---|---|
| [`symbol_new_for_short_literal`](symbol_new_for_short_literal.md) | `warn` | Medium Impact | `Symbol::new` with short literal arguments |

## User Overrides

All default levels can be overridden per-workspace via `budget.toml` — see the [Integration Guide](../integration.md).

### Breaking Change Notice

Changing a default level from `warn` to `deny` (as done for `soroban_storage_in_loop` in v0.2.0) **may break existing CI pipelines** that run with `-D warnings` or treat warnings as errors. Projects that relied on the previous `warn` default should either:

- Add `#[allow(soroban_storage_in_loop)]` to deliberate call sites, or
- Set `soroban_storage_in_loop = "warn"` in `budget.toml` to restore the old level.
