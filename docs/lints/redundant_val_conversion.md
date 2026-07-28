# `redundant_val_conversion`

**Default Severity:** `warn`  
**Category:** Compute

## What it does

Detects unnecessary conversions to and from Soroban's `Val` type. Specifically, it flags:
1. Converting a value into a type it already is (e.g. converting a `u32` to a `u32` via `.into_val()`).
2. Round-trip conversions where a value is converted into a `Val` and immediately back into its original type within the same expression chain (e.g. `u32::try_from_val(env, &num.into_val(env))`).

## Why is this bad?

In Soroban, converting cross-boundary types between native Rust types and the host environment's `Val` type requires metered host calls. 
A round-trip conversion produces the exact same value you started with but consumes unnecessary compute gas along the way. These often accumulate quietly when passing values between helper functions.

## Example

**Bad:**
```rust
let num = 5u32;
let same_num = u32::try_from_val(env, &num.into_val(env));
```

**Good:**
```rust
let num = 5u32;
let same_num = num;
```

## Known False Positives & Limitations

This lint strictly analyzes direct inline expression chains to avoid false positives related to generic bounds and macros. 
It will not flag round-trip conversions that occur across multiple statements due to dataflow separation.
```rust
// This multi-statement round-trip is currently NOT flagged
let val: Val = num.into_val(env);
let same_num = u32::try_from_val(env, &val);
```
