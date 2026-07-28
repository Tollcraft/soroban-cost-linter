# Phase 1: Reconnaissance Analysis — Issue #206 / Task #18

## Executive Summary

This document captures the reconnaissance phase analysis for resolving the issue:
**"test: increase unit test coverage for edge scenarios #18"**

Currently, the issue description contains template placeholders rather than specific requirements. This analysis documents the repository structure, testing patterns, and clarifications needed to proceed with implementation.

---

## Issue Requirements (As Provided)

From the original issue description:

- **Goal**: Increase unit test coverage for edge scenarios
- **What "done" looks like**:
  - Add tests that specifically target off-by-one errors and zero-length inputs
  - Ensure code is clean and passes all checks
  
- **Implementation guidelines**:
  - Create a branch: `git checkout -b test/task-18` ✅ **DONE**
  - Key file: `src/module_18.rs` ⚠️ **Does not exist** — placeholder value
  - Ensure new tests are deterministic

- **PR requirements**:
  - PR description must include: `Closes #18`
  - Follow code quality standards (cargo fmt, clippy, cargo test)

---

## Repository Structure Analysis

### Linting Architecture
This is a **Dylint-based linting tool** for Soroban (Stellar smart contract platform). It implements cost-aware lints that detect anti-patterns in Soroban SDK code.

### Current Lints Implemented

The repository implements 9 distinct lints:

1. **SOROBAN_STORAGE_IN_LOOP** — Storage operations inside loops
2. **REDUNDANT_ENV_CLONE** — Redundant cloning of Env objects (Memory)
3. **UNNECESSARY_HOST_FUNCTION_CALL** — Repeated calls to host functions with constant inputs
4. **HOST_IN_LOOP** — Host object method calls in loops (Compute)
5. **SYMBOL_NEW_FOR_SHORT_LITERAL** — Symbol creation from short literals
6. **STORAGE_WRITE_WITHOUT_READ** — Write-only storage patterns (StorageOperations)
7. **INEFFICIENT_BYTES_CONCAT** — Inefficient byte concatenation (Memory)
8. **MAP_INSERT_IN_LOOP** — Map insertions inside loops (StorageOperations)
9. **BYTES_APPEND_IN_LOOP** — Bytes container growth in loops (Memory)

All lints are registered in: [soroban_cost_lints/src/lib.rs](soroban_cost_lints/src/lib.rs)

### Test Structure

**Test Framework**: Dylint UI tests (snapshot/expected output tests)

**Main test file**: [soroban_cost_lints/ui/main.rs](soroban_cost_lints/ui/main.rs)
- Contains test case structs and mock Soroban SDK implementations
- Tests are written as Rust code that should/should not trigger lints
- Expected lint output is captured in: [soroban_cost_lints/ui/main.stderr](soroban_cost_lints/ui/main.stderr)

**Test execution**:
```bash
cargo test --workspace           # Run all tests
cargo fmt --all                  # Format code
cargo clippy --workspace --all-targets -- -D warnings  # Lint checks
```

### Files That Will Be Modified

Based on repository structure, the following files would need updates:

1. **[soroban_cost_lints/ui/main.rs](soroban_cost_lints/ui/main.rs)** — Add edge case test code
2. **[soroban_cost_lints/ui/main.stderr](soroban_cost_lints/ui/main.stderr)** — Update expected lint output
3. **[soroban_cost_lints/src/lib.rs](soroban_cost_lints/src/lib.rs)** — Potentially improve lint logic if needed

---

## Testing Patterns Discovered

### Existing Test Style (from main.rs)

```rust
// Pattern 1: Tests that SHOULD trigger the lint
#[warn(soroban_storage_in_loop)]
pub fn storage_in_loop(env: Env) {
    let storage = env.storage();
    for _ in 0..10 {
        storage.instance().set(&0, &1);  // ⚠️ Warning triggered
    }
}

// Pattern 2: Tests that should NOT trigger the lint
#[allow(soroban_storage_in_loop)]
pub fn storage_outside_loop(env: Env) {
    let storage = env.storage();
    storage.instance().set(&0, &1);  // ✓ No warning
}
```

### Edge Cases for Each Lint Category

**Off-by-one errors**:
- Loop with `i < n-1` vs `i < n`
- Range iterations with exclusive vs inclusive bounds

**Zero-length inputs**:
- Empty collections (Vec, Map, Bytes)
- Zero-iteration loops
- Empty iterators

**Boundary conditions**:
- Single-element collections
- Loops that execute exactly once
- Maximum size boundaries

---

## Critical Clarification Needed

### ❓ Which Lint(s) to Test?

The original issue mentions `src/module_18.rs`, which doesn't exist. We need to determine:

- [ ] Should we add edge cases for **all 9 lints** or specific ones?
- [ ] Should we prioritize lints with **highest runtime impact** (storage operations)?
- [ ] Are there specific lints already known to have **untested edge cases**?

### ❓ What Specific Edge Cases Matter?

For different lints, relevant edge cases differ:

**Storage & Loop Lints** (SOROBAN_STORAGE_IN_LOOP, MAP_INSERT_IN_LOOP, BYTES_APPEND_IN_LOOP):
- Zero-iteration loops
- Single-iteration loops
- Nested loops
- Conditional loops (while vs for)

**Clone/Redundancy Lints** (REDUNDANT_ENV_CLONE):
- Multiple clones in sequence
- Clones in different scopes
- Clones with type conversions

**Host Function Lints** (UNNECESSARY_HOST_FUNCTION_CALL, HOST_IN_LOOP):
- Calls with loop-dependent arguments
- Calls with constant arguments
- Calls in iterator closures

**Symbol/Literal Lints** (SYMBOL_NEW_FOR_SHORT_LITERAL):
- Single-character symbols
- Multi-byte UTF-8 symbols
- Very long literals

---

## Recommended Next Steps

### **Immediate Action (BLOCKING)**

**We need clarification from the issue stakeholders:**

1. **Which lint(s) need edge case testing?** 
   - Suggest: Start with `BYTES_APPEND_IN_LOOP` and `SOROBAN_STORAGE_IN_LOOP` (highest cost impact)

2. **What are the critical edge cases?**
   - Example: "Test that zero-length byte appends don't trigger false positives"

3. **Coverage targets:**
   - What code coverage % is required?
   - Are there known gaps in current tests?

### **After Clarification (PHASE 2-4)**

Once clarified, execution will follow the strict 4-phase workflow:
- **PHASE 2**: Implement test cases for edge scenarios
- **PHASE 3**: Validate all new tests pass and existing tests remain unchanged
- **PHASE 4**: Run full quality checks (fmt, clippy, test) and commit

---

## Code Quality Standards (From CONTRIBUTING.md)

All PRs must pass:

```bash
# Format check
cargo fmt --all -- --check

# Linting (no warnings)
cargo clippy --workspace --all-targets -- -D warnings

# Full test suite
cargo test --workspace
```

---

## Lint Metadata Reference

All lints are categorized by cost impact:

| Lint | Category | Impact |
|------|----------|--------|
| SOROBAN_STORAGE_IN_LOOP | StorageOperations | 🔴 HIGH |
| BYTES_APPEND_IN_LOOP | Memory | 🟡 MEDIUM |
| MAP_INSERT_IN_LOOP | StorageOperations | 🔴 HIGH |
| HOST_IN_LOOP | Compute | 🟡 MEDIUM |
| SYMBOL_NEW_FOR_SHORT_LITERAL | SymbolOperations | 🟢 LOW |
| STORAGE_WRITE_WITHOUT_READ | StorageOperations | 🔴 HIGH |
| INEFFICIENT_BYTES_CONCAT | Memory | 🟡 MEDIUM |
| REDUNDANT_ENV_CLONE | Memory | 🟡 MEDIUM |
| UNNECESSARY_HOST_FUNCTION_CALL | Compute | 🟡 MEDIUM |

---

## Branch Status

- ✅ Branch created: `test/task-18`
- ✅ Upstream synced
- ⏳ Awaiting clarification to proceed with implementation

---

## Document Metadata

- **Created**: 2026-07-26
- **Branch**: test/task-18
- **Status**: Reconnaissance complete, awaiting stakeholder clarification
- **Next Step**: PHASE 1 approval and clarification responses

---

## How to Use This Document

This document should be included in the PR to:
1. **Explain the analysis** conducted during reconnaissance
2. **Identify gaps** in the original issue template
3. **Propose concrete next steps** for implementation
4. **Provide context** for reviewers about testing strategy

**Questions for reviewers?** See the "Critical Clarification Needed" section above.
