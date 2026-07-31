# host_in_loop

## What it does

This lint flags uses of the Soroban host object inside loops. Repeated host access can significantly increase runtime cost because the host interaction is expensive and often should be moved outside the loop.

## Why is this bad

Performing host object access inside a loop multiplies the cost of each host call by the number of loop iterations. Each host interaction burns CPU and memory budget, so repeating it in a loop can quickly exhaust the contract's resource limits, leading to aborted transactions.

## How to fix it

Move the host object access outside the loop and reuse the result, or restructure the code to batch the host operations into a single call after the loop.

## Default level

Warn
