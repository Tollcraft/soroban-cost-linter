# `unnecessary_host_function_call_legacy`

| Property | Value |
| --- | --- |
| Default severity | `warn` |
| Category | Host |

## What it does

This is a legacy lint retained for backward compatibility. It detects unnecessary host function calls inside loop bodies.
See [`unnecessary_host_function_call`](unnecessary_host_function_call.md) for the active implementation and details.

## Why is this bad?

{% hint style="danger" %}
Host function calls across the Wasm-host boundary incur host dispatch overhead and resource consumption. When performed unnecessarily or repeatedly with identical inputs, they cause avoidable CPU and fee overhead.
{% endhint %}

