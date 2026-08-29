# Severity Levels and the `soroban_storage_in_loop` Default

> This page explains what the three severity levels mean, why exactly one lint defaults to `deny` while every other shipped lint defaults to `warn`, and how to choose severities for your own project.

---

## The Three Severity Levels

`soroban-cost-linter` uses the same three severity levels as the Rust compiler:

| Level | What it does | Build outcome |
|-------|-------------|---------------|
| **allow** | Suppresses the lint entirely — the diagnostic is never emitted. | Build succeeds (lint is invisible). |
| **warn** | Emits a warning during the build. The compiler proceeds and exits successfully. | Build succeeds, but warnings appear in output. |
| **deny** | Emits a hard error. The compiler stops and the build fails. | **Build fails.** No warning — the code must be fixed or the lint suppressed. |

When you run `cargo cost-lint`, every lint runs at its **built-in default** unless you override it with a `budget.toml` file. See the [Integration Guide](integration.md) for how `budget.toml` works.

### Precedence

Rustc resolves the effective lint level in this order (highest priority first):

1. `--force-warn` / `--cap-lints` flags (not set by this tool)
2. `#[allow]` / `#[warn]` / `#[deny]` attributes in source code
3. Compiler flags from `DYLINT_RUSTFLAGS` (the mechanism used by `budget.toml`)
4. The lint's built-in default

This means a `deny` in `budget.toml` overrides the default, and an `#[allow]` attribute in source code can suppress a `warn`-level lint for a specific function — but **cannot** override a `deny` from `budget.toml`.

---

## Why `soroban_storage_in_loop` Defaults to `deny`

This linter ships **24 lints**. Twenty-three default to `warn`. Exactly one — `soroban_storage_in_loop` — defaults to `deny`.

That is a deliberate decision, not an oversight. The reasoning comes directly from the [Cost Rationale](cost_rationale.md):

### Storage is the most expensive resource

Storage operations — ledger entry reads and writes — are the **single most expensive resource** a Soroban contract can consume. They dominate the resource fee hierarchy:

| Rank | Operation | Primary resource consumed |
|------|-----------|---------------------------|
| 1 (most expensive) | **Storage writes** | Ledger entry writes + I/O bytes |
| 2 | **Storage reads** | Ledger entry reads + I/O bytes |
| 3 | Host function calls | CPU (dispatch + function work) |
| 4 | Wasm arithmetic / control flow | CPU (WasmInsnExec) |
| 5 (least expensive) | Memory operations | RAM (capped, not charged) |

A single storage write in a loop can cost more than the rest of the loop body combined, because it consumes four resources simultaneously: a ledger entry access, write I/O bytes, the serialization CPU cost, and (for new entries) space rent.

### Multiplying storage by a loop count is the most expensive pattern the tool detects

Every other lint in this repository catches patterns that are *expensive* — but none of them multiply the single most expensive resource by an unbounded loop count. That combination is what makes `soroban_storage_in_loop` categorically different:

- A redundant `Env` clone wastes some CPU, but CPU is cheaper than storage.
- An unnecessary host function call in a loop wastes some dispatch overhead, but host calls are cheaper than storage.
- `bytes_append_in_loop` causes allocations, but memory is the least expensive resource.

A storage write in a loop, by contrast, is the most expensive structural pattern the tool can identify. Making it `deny` reflects that this pattern is almost never intentional and almost always indicates a bug or a design mistake that will blow through the transaction budget.

### Why not `deny` everything else?

The remaining 23 lints flag patterns that are *inefficient* but not necessarily *budget-breaking*. A `warn` is appropriate because:

1. **Context matters.** A host function call inside a loop is wasteful *unless* it's reading ledger state that changes each iteration. The tool cannot always prove the call is unnecessary.
2. **False positives are possible.** Some detections are heuristic (e.g., `linear_scan_in_loop`). A hard failure would be frustrating for code that is actually fine.
3. **Warn gives users a signal without blocking CI.** A team can review warnings on their schedule rather than being forced to fix every finding before the build passes.

---

## Choosing Severities for Your Own Project

### When to promote a lint to `deny`

Consider promoting a lint to `deny` in your `budget.toml` when **all three** of these are true:

1. **The pattern is almost never correct.** You cannot think of a reasonable situation where you would intentionally write that code.
2. **The cost impact is severe.** The pattern will meaningfully increase your transaction's resource fee — not just marginally.
3. **Your team agrees.** A `deny` means CI fails. Everyone on the team should understand what triggers it and how to fix it.

Good candidates for `deny` in cost-sensitive projects:

| Lint | Why it's a good `deny` candidate |
|------|--------------------------------|
| `soroban_storage_in_loop` | Already `deny` by default — storage × loop is the most expensive pattern. |
| `extend_ttl_in_loop` | TTL extension is a metered storage operation; calling it per-iteration wastes both CPU and rent. |
| `contract_call_in_loop` | Each cross-contract call spins up a new VM context; the overhead multiplies with loop count. |
| `signature_verification_in_loop` | Signature verification is one of the most expensive host functions available. |

### When to keep a lint at `warn`

Keep a lint at `warn` when:

- The pattern is *usually* inefficient but *sometimes* correct.
- The tool's detection is heuristic and may produce false positives.
- The cost impact is moderate — noticeable but not budget-breaking.

### When to use `allow`

Use `allow` for lints that:

- Flag patterns your codebase uses intentionally (e.g., a deliberate `Env` clone in a specific context).
- Apply to third-party code you don't control.
- Generate noise for your specific project structure.

---

## The Rule for New Lints

When a contributor adds a new lint to this repository, the default should be **`warn`** — unless the lint catches a pattern that is:

1. **Categorically expensive** — it multiplies the most expensive resource (storage) by an unbounded count.
2. **Almost never correct** — there is no reasonable scenario where the flagged code is intentional.

In practice, this means: **everything defaults to `warn` unless it is storage-in-a-loop.**

This is a deliberately narrow exception. If a future lint catches something that is arguably as expensive as storage-in-a-loop, the case for `deny` should be made explicitly in the lint's own PR, with evidence from the [Cost Rationale](cost_rationale.md). A short honest rule beats an invented taxonomy.

---

The [Lint Catalog](lint_catalog.md) is generated directly from the source `declare_lint!` definitions via `tools/generate-lint-docs` and reflects exact default severities (such as `soroban_storage_in_loop` defaulting to `deny`).
