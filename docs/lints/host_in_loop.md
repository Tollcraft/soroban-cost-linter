# host_in_loop

## What it detects

This lint flags uses of the Soroban host object inside loops. Repeated host access can significantly increase runtime cost because the host interaction is expensive and often should be moved outside the loop.

## Why it matters

Moving host access out of loops reduces unnecessary host round-trips and helps keep contract execution costs predictable.

## Default level

Warn
