# unnecessary_host_function_call_legacy

| Property | Value |
| --- | --- |
| Default severity | `warn` |
| Category | Host |

## What it does

Flags the same pattern as [`unnecessary_host_function_call`](unnecessary_host_function_call.md): host-function calls that could be hoisted out of a loop or avoided altogether.

This is a legacy alias retained so existing `allow`/`deny` configuration keeps working. New code should configure `unnecessary_host_function_call` instead.

## Why is this bad?

Every host-function call crosses the VM boundary and is charged for separately, so a call that is repeated or avoidable is paid for on every invocation for no benefit.

## Category

Host
