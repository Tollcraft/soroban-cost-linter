# host_in_loop

## What it does

This lint flags uses of the Soroban host object inside loops. Repeated host access can significantly increase runtime cost because the host interaction is expensive and often should be moved outside the loop.

## Why is this bad

Performing host object access inside a loop multiplies the cost of each host call by the number of loop iterations. Each host interaction burns CPU and memory budget, so repeating it in a loop can quickly exhaust the contract's resource limits, leading to aborted transactions.

## How to fix it

Move the host object access outside the loop and reuse the result, or restructure the code to batch the host operations into a single call after the loop.

## Related lints

- **[`ledger_context_read_in_loop`](ledger_context_read_in_loop.md):** A more specific lint that flags ledger context reads (`sequence`, `timestamp`, `network_id`) inside loops and explains that the value is invariant during the invocation. A ledger read in a loop may trigger both lints; the `ledger_context_read_in_loop` message provides the more targeted fix.

## Default level

Warn
