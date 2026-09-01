# u128_where_u64_suffices

| Property | Value |
| --- | --- |
| Default severity | `warn` |

## What it does

Uses 128-bit arithmetic where 64 bits would suffice, which is extremely expensive on wasm32.

## Why is this bad?

wasm32 lacks native 128-bit integer instructions; emulating them is very slow.

## Category

Compute

