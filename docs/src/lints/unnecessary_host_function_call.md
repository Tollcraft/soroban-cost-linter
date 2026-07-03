# `unnecessary_host_function_call`

**Default Severity:** Warn

## What it does
Identifies redundant calls to host functions, such as fetching the ledger sequence or timestamp, inside loop bodies.

## Why is this bad?
Calling a host function crosses the boundary into the Soroban environment, which incurs a CPU cost. Repeating this unnecessarily inside a loop adds up to significant waste, especially when the result is constant across iterations.

## Example
```rust
for item in items {
    let current_seq = env.ledger().sequence(); // Bad
}
```

## Suggested Fix
Call the host function once before the loop, store the result in a local variable, and reference that variable inside the loop.
