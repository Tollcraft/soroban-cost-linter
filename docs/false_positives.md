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

### `unbounded_recursion`

This lint flags a recursive call cycle (direct or mutual) whose depth is driven by
caller-supplied input — a caller-supplied `Vec`/`&[T]` length, a tail slice, or a
slicing/`to_vec` operation on caller data. False positives and accepted gaps:

- **Structurally-bounded recursion reported as unbounded:** a collection consumed by a method *not* in the recognized tail set (e.g. a custom `fn rest(&self) -> Self` returning a strict sub-slice) may not be recognized as progress. Prefer `#[allow(unbounded_recursion)]` for such intentional, provably-bounded cases.
- **Constant-argument "infinite-looking" recursion:** `fn f(n: u32) { if n == 0 { return; } f(3); }` passes a constant argument, so the lint treats it as bounded and stays silent even though `n` never decreases. The lint keys off the *argument shape*, not the actual termination proof, to stay sound and simple.
- **Plain integer parameters threaded through the recursion:** `fn process(n: u32) { if n == 0 { return; } process(n - 1); }` is structurally a countdown, but the *initial* value of `n` is caller-supplied, so the depth is not provably constant. The lint treats it as *unknown* and stays silent.
- **Recursion through trait objects, function pointers, or closures:** the call target cannot be resolved to a single local `DefId`, so these calls are never recorded as graph edges and never form a cycle the lint reports.
- **Recursion whose bound the analysis cannot determine:** generics, complex control flow, or arguments that are neither a constant nor a caller-supplied collection with a tail operation are all left alone.

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

### `loop_invariant_storage_access`

This lint flags storage method calls (`env.storage()`, `.instance()`/`.persistent()`/`.temporary()`, and the terminal `get`/`has`/`set`) that sit inside a loop and whose operands are provably loop-invariant. Notes from writing the fixture:

- A single logical `env.storage().instance().get(&1)` inside a loop emits **three** warnings — one per call in the chain (`storage`, `instance`, then `get`) — because each call is matched independently and each is loop-invariant. This is expected, not a defect.
- **Genuine near-miss (must not fire):** when the *receiver* is the loop variable — `for s in stores.iter() { s.get(&1); }` where `s: &Instance` — the access depends on loop state and is correctly skipped. This is the real "varies per iteration" case.
- **Subtler near-miss:** a `get(item)` whose *argument* `item` is the loop variable. The `get` call is correctly skipped (its argument depends on loop state), but the `env.storage()` and `.instance()` receiver calls are *still* flagged, because their receiver `env` is constant and therefore loop-invariant. So "the key varies per iteration" does not fully silence the lint — only the terminal call is suppressed. This is documented so the surviving receiver warnings are not mistaken for a bug.
- The lint keys off structural loop-invariance, not value ranges; a literal key (`&1`) is treated the same as `let k = &1; get(k)`.

### `soroban_redundant_storage_read`

Fires when two reads of the same key (by source-text snippet) appear with no intervening write, at the top level of a block.

- **Near-miss 1 — write between reads:** `get(&1); set(&1, &2); get(&1)`. The write resets the tracked key, so the second read is **not** flagged. Correct.
- **Near-miss 2 — different keys:** `get(&1); get(&2)`. Different source-text keys, so no redundancy is reported. Correct.
- Key equality is **textual** (the `snippet_opt` of the key argument), not semantic. `get(&k)` and `get(&k)` match; `get(&1)` and `get(1)` (without the `&`) would not, because the snippets differ. The check is syntactic — keep this in mind when reading the fixture.
- The lint only compares reads that are top-level statements/expressions within the **same** block. A read inside a nested `if`/`match`/closure is in a different block and is not compared against an outer-block read of the same key; that is why the fixture keeps both reads at the same block level.
- **No known false positives:** in every case the warning corresponds to a real duplicate read.

### `storage_write_without_read`

Fires on any `set` whose `(receiver, key)` snippet has no matching `get`/`has` anywhere in the same function.

- **Near-miss — initializer skip:** a function whose name contains `init` or `set_admin` is intentionally not analyzed, so a legitimate initializing `set` with no prior read stays silent. The fixture exercises this with `fn initialize(...)` and `fn set_admin(...)`.
- **Near-miss 2 — read precedes write:** `get(&1); set(&1, &2)`. The prior read of the same key suppresses the warning. Correct.
- Matching is by source-snippet text, so a read written with a syntactically different but semantically equal key (e.g. `has(&key)` paired with `set(key)` without the `&`) will not link and the write will still fire. The fixture uses identical snippets to exercise the matching path.
- Analysis is **per-function**: reads in a different function do not count toward a write's read set.
- **No known false positives** beyond the intentional initializer skip: any write with a truly absent read is reported, which is the lint's purpose.

### `persistent_read_without_ttl_extension`

Fires on every `get`/`has` on `persistent` storage when the function contains no `extend_ttl` call.

- **Near-miss 1 — TTL extended:** a single `extend_ttl(...)` call anywhere in the function suppresses **all** persistent-read warnings for that function. The check is all-or-nothing per function, not per key — so a function that extends TTL on one key but reads others without extending still produces no warning. Documented because it is easy to misread the fixture as per-key.
- **Near-miss 2 — non-persistent storage:** `instance.get(...)` / `temporary.get(...)` are out of scope and never flagged.
- The lint collects reads via a visitor over the whole function body, so a read in a nested block still counts.
- **No known false positives:** a persistent read with no `extend_ttl` in the function is always reported.

### `instance_storage_write_in_loop`

Fires on every `.instance().set(...)` call that sits inside a loop body.

- **Interaction with `soroban_storage_in_loop`:** both lints fire on the same expression when instance storage is written inside a loop. `soroban_storage_in_loop` covers storage in general; `instance_storage_write_in_loop` fires specifically because instance storage serialises and rewrites the full entry map on every write, making the fix different (accumulate in locals, write once after the loop) from the general "move storage operations out of the loop" advice.
- **Known near-miss — batch writes to different keys:** `for (k, v) in pairs { env.storage().instance().set(k, v); }` writes a different key each iteration. The full-instance rewrite still happens per iteration, so the cost is real — this is a genuine warning, not a false positive.
- **No false positives from reads:** the lint only matches `set`, so `get` and `has` calls on instance storage inside a loop are never flagged by this lint (they are covered by other lints if applicable).
- **No false positives from other storage types:** `Persistent` and `Temporary` writes inside loops are out of scope — those are per-entry stores, not the single-blob instance map.

### `unwrap_on_storage_get`

Fires on `.unwrap()` / `.expect()` called *directly* on a storage `get` (on `Storage`, `Instance`, `Persistent`, or `Temporary` receivers).

- **Read the contract has genuinely just written:** `env.storage().instance().set(&key, &value);` followed later by `env.storage().instance().get(&key).unwrap()`. Within one invocation the key was just written, so the read cannot miss and the unwrap cannot trap. The lint has no write-tracking across statements, so this shape still fires even though it is provably safe at runtime — suppress with `#[allow(unwrap_on_storage_get)]` or handle the `None` case anyway if the write can ever be removed.
- **Anything not directly on a storage read is out of scope by construction:** `unwrap_or`, `unwrap_or_else`, an explicit `match` on the returned `Option`, and `unwrap` on any `Option`/`Result` that did not come from a storage `get` never fire.
