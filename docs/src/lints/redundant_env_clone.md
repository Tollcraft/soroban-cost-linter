# `redundant_env_clone`

**Default Severity:** Warn

## What it does
Detects unnecessary `.clone()` calls on the Soroban `Env` object.

## Why is this bad?
The Soroban `Env` object is designed to be highly lightweight and is typically passed by value or reference. Cloning it incurs unnecessary CPU cycles.

## Example
```rust
let my_env = env.clone(); // Bad
```

## Suggested Fix
Pass `env` directly by value or reference without calling `.clone()`.
