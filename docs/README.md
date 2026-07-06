# ⚡ soroban-cost-linter

> **The static analysis shield for Soroban smart contracts** — catch structurally expensive patterns before they are compiled and deployed to the network.

Part of the **Tollcraft** initiative.

## 💸 The Cost of Soroban Operations

Soroban charges for every resource your contract consumes:

| Resource                    | Cost profile                                                 |
| --------------------------- | ------------------------------------------------------------ |
| 🧮 **CPU instructions**     | Charged per instruction executed                             |
| 🧠 **Memory allocations**   | Charged per allocation                                       |
| 💾 **Storage operations**   | Reads and writes — often the **most expensive** component    |

Storage operations (reads and writes) are often the most expensive component of any smart contract transaction. For instance, executing repeated storage operations inside a loop rather than aggregating state changes in memory before writing can significantly increase the total budget utilized.

{% hint style="warning" %}
**Every wasted instruction is a fee your users pay.** Cost bugs don't fail tests — they silently inflate transaction fees in production.
{% endhint %}

## 🛡️ How It Works

Our linter hooks directly into the Rust compiler's High-Level Intermediate Representation (HIR) via [Dylint](https://github.com/trailofbits/dylint) to detect input-independent, structurally expensive patterns — alerting you before they are compiled and deployed to the network.

## 🧭 Jump In

{% hint style="info" %}
New here? Start with the **[Integration Guide](integration.md)** to wire the linter into your workspace and CI in minutes.
{% endhint %}

* 🔍 **[Lint Reference](lints/README.md)** — every lint, what it catches, and how to fix it
  * [`soroban_storage_in_loop`](lints/soroban_storage_in_loop.md)
  * [`redundant_env_clone`](lints/redundant_env_clone.md)
  * [`unnecessary_host_function_call`](lints/unnecessary_host_function_call.md)
* 🔌 **[Integration Guide](integration.md)** — `budget.toml` configuration and GitHub Actions setup
