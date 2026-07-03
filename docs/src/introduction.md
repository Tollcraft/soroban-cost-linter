# Introduction

Welcome to `soroban-cost-linter`, the static analysis shield for Soroban smart contracts, part of the Tollcraft initiative.

## The Cost of Soroban Operations
Soroban charges for CPU instructions, memory allocations, and storage operations. Storage operations (reads and writes) are often the most expensive component of any smart contract transaction. For instance, executing repeated storage operations inside a loop rather than aggregating state changes in memory before writing can significantly increase the total budget utilized.

Our linter hooks directly into the Rust compiler's High-Level Intermediate Representation (HIR) via Dylint to detect these input-independent, structurally expensive patterns, alerting you before they are compiled and deployed to the network.
