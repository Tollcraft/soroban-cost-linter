# Handling False Positives

Static analysis tools occasionally flag code that is intentionally written the way it is. This guide explains how to recognize, suppress, and report false positives in `soroban-cost-linter`.

## What is a False Positive?

A false positive is a lint warning that fires on code that does not actually contain the problem the lint is designed to catch.

For example, `soroban_storage_in_loop` warns when a storage operation appears inside a loop body. In most code this is an expensive anti-pattern, but if you are intentionally writing different keys on each iteration (e.g., writing a batch of entries), the warning is a false positive — the code is correct, and the cost is inherent to the operation.

## Known False Positive Patterns by Lint

### `soroban_storage_in_loop`

Every storage read or write inside any loop body is flagged. This is correct for the dominant case, but false positives arise when:

- **Batch writes with different keys** — iterating over a collection and writing each element under a different storage key.
- **Storage reads that depend on the loop variable** — reading a value for each item in a collection, where the key changes per iteration.
- **Counting or scanning patterns** — using a loop to count entries or scan through storage with `has()`.

The lint does not analyse whether the key changes between iterations; it errs on the side of reporting.

### `unnecessary_host_function_call`

This lint uses mutation analysis to leave calls alone when their arguments depend on loop state. Known gaps that produce false positives:

- **Bindings and mutations inside a closure body** nested in the loop are not tracked.
- **Mutation through a raw pointer or interior mutability** (`Cell`, `RefCell`) is not tracked.
- **Intentional per-iteration calls** like `env.prng().u64_in_range()` or `env.events().publish()` with constant arguments are still reported — the lint cannot distinguish intent from waste.

### `redundant_env_clone`

This lint fires for every `.clone()` call on `Env`. False positives occur when:

- The `Env` is consumed before the clone site and you genuinely need a second handle.
- The code is generic over a trait that does not guarantee `Env`-like cheap pass-by-value semantics.

### `symbol_new_for_short_literal`

This lint fires when `Symbol::new(&env, literal)` is called with a short literal. False positives occur when:

- The literal is constructed dynamically (non-literal argument) — the lint already handles this.
- The macro `symbol_short!` is unavailable in your environment (e.g., an older SDK version).

## Suppression Methods

You have three layers of suppression, each suited to a different scope.

### 1. Per-site: `#[allow(...)]` Attribute

Suppress the lint for a single function, expression, or block:

```rust
#[allow(soroban_storage_in_loop)]
fn batch_write(env: Env, items: Vec<u32>) {
    for item in items {
        env.storage().instance().set(&item, &1);
    }
}
```

This is the most targeted suppression. Use it when the flagged code is intentional and the lint gives no other way to express that intent.

You can also use `#[expect(...)]` (nightly Rust) to suppress and verify that the lint fires — the compiler will warn if the lint _stops_ firing, which is useful when a future version of the lint might no longer flag the pattern:

```rust
// Will warn if soroban_storage_in_loop no longer fires on this code
#[expect(soroban_storage_in_loop)]
fn batch_write(env: Env, items: Vec<u32>) {
    for item in items {
        env.storage().instance().set(&item, &1);
    }
}
```

### 2. Per-file: `.lintignore`

Create a `.lintignore` file in your workspace root (next to `Cargo.toml`). The linter respects the same patterns as `.gitignore`:

```gitignore
# Ignore all lint warnings in a generated file
src/generated/constants.rs

# Ignore a deliberately expensive module
src/costly_but_intentional.rs
```

Entries in `.lintignore` cause every lint finding in matching files to be silently dropped. This is useful for generated code, vendored dependencies, or files where you have decided the lint does not apply.

### 3. Per-workspace: `budget.toml`

Set a lint's severity to `"allow"` in `budget.toml` to suppress it project-wide:

```toml
[lints]
soroban_storage_in_loop = "allow"
```

This is the broadest suppression. Use it sparingly — it disables the lint for the entire workspace. Prefer `#[allow(...)]` or `.lintignore` when you need to suppress only specific sites.

## How to Evaluate a False Positive

Before suppressing, ask:

1. **Is the cost real?** — Does removing the warning require changing the algorithm, or is it just adding an attribute? If the cost is inherent to what the code does, suppress. If the code can be restructured to avoid the cost, fix it instead.
2. **Is the pattern covered by a different lint?** — For example, a storage read inside a loop that depends on the loop variable is real work. But a storage write inside a loop that writes the same key on every iteration is a bug.
3. **Is there a Clippy lint that handles this better?** — Some patterns that `soroban-cost-linter` flags may be general Rust inefficiencies already caught by Clippy. See the [Scope Boundary](scope_boundary.md) guide.

## Reporting False Positives Upstream

If a lint produces a false positive that cannot be worked around with the suppression methods above, please open an issue:

1. Check existing issues to see if the pattern is already reported.
2. Include a minimal reproduction — a self-contained Rust function that triggers the false positive.
3. State which lint fired and why the code is correct despite the warning.
4. Mention the `soroban-cost-linter` version and the Rust toolchain version.

The lint's mutation analysis (used by `unnecessary_host_function_call`) is the area most likely to improve; regression tests from real-world false positives are particularly valuable.

## Verifying Suppression in Tests

When you suppress a lint, verify that the suppression works correctly:

1. **With `#[allow(...)]`** — compile with the attribute. The lint should not fire. Remove the attribute and confirm the lint does fire (to prove the code would have been flagged).
2. **With `.lintignore`** — run `cargo cost-lint` with and without the `.lintignore` entry to confirm the finding appears or disappears.
3. **With `budget.toml`** — set the level to `"allow"` and confirm `cargo cost-lint` exits with code 0 even when the pattern is present.

## Summary

| Scope | Method | Best for |
|-------|--------|----------|
| Per-site | `#[allow(lint_name)]` | Intentional patterns at specific call sites |
| Per-file | `.lintignore` | Generated code, vendored deps, entire files |
| Per-workspace | `budget.toml` `"allow"` | Project-wide decisions (use sparingly) |
