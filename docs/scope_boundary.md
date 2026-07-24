# Scope: Clippy vs. `soroban-cost-linter`

This linter runs on the same compiler infrastructure as Clippy and shares `clippy_utils`. Several patterns in the backlog have direct Clippy equivalents. This page fixes the boundary between the two tools so new lint proposals can be evaluated consistently.

## The Rule

`soroban-cost-linter` covers **Soroban-specific, host-metered cost patterns** — anti-patterns whose harm is measurable in the Soroban resource model (CPU instructions, memory allocations, storage reads/writes, and host-function crossings).

Clippy covers **general Rust inefficiency and non-idiomatic code** — patterns that are wasteful or confusing regardless of the target runtime.

If a pattern is already caught by a Clippy lint for general Rust, do not duplicate it here unless the Soroban cost consequence is materially different and the existing Clippy lint does not already cover the Soroban-specific harm.

{% hint style="info" %}
The two tools are complementary. Run both in CI.
{% endhint %}

## Patterns Deliberately Left to Clippy

These patterns are already covered by named Clippy lints. Do not re-implement them in this crate:

| Soroban pattern | Clippy lint | Why we defer |
| --- | --- | --- |
| Cloning a `Copy` type, or any clone that is redundant in the ownership sense | `clippy::redundant_clone` | General Rust waste; no Soroban-specific angle |
| Calling `.clone()` on a `Copy` type | `clippy::clone_on_copy` | Trivially redundant regardless of runtime |
| Collecting an iterator into a collection that is never used as a collection | `clippy::needless_collect` | Unnecessary allocation on any target |
| Redundant `Into` / `From` / `IntoIterator` conversions | `clippy::useless_conversion` | Pure type-system waste; cost-neutral on Soroban |

## The Genuine Overlap: When to Write a Distinct Soroban Lint

A Soroban-specific lint is justified when Clippy's equivalent does not capture the **host-metered cost dimension**. The test is:

> Does the Soroban cost model make this pattern harmful in a way that general Rust does not?

If yes, write the Soroban lint and reference the Clippy lint as the "why we don't just leave this to Clippy" justification.

## Worked Example: `redundant_env_clone` vs. `clippy::redundant_clone`

`redundant_env_clone` is the current boundary case and the model for evaluating future proposals.

### What Clippy covers

`clippy::redundant_clone` fires when a `.clone()` result is unused or when the original value is still available without the clone. It is an ownership-level check: "you cloned but did not need to."

### What Soroban adds

The Soroban `Env` object is not a plain Rust value. Cloning it can cross the guest/host boundary or trigger host-side bookkeeping that Soroban meters. Even when a clone is **not redundant from Clippy's perspective** (the original is consumed, the clone is genuinely needed), it may still be **costly in a way the developer did not intend**.

`redundant_env_clone` therefore targets a different question:

> Is this clone on `Env` unnecessary **given how `Env` is passed in this function**, regardless of whether the borrow checker would allow removing it?

### Example

```rust
// Clippy sees: `env` is still available after the clone, so `cloned` is redundant.
// `clippy::redundant_clone` may or may not fire here depending on the exact flow.

fn handle(env: Env, items: Vec<Item>) {
    for item in items {
        let cloned = env.clone(); // soroban-cost-linter flags this
        process(cloned, item);
    }
}
```

In Soroban, `Env` is cheap to pass by value. Cloning it per-iteration is an avoidable host crossing. General Rust has no concept of "host crossing cost," so Clippy cannot express the warning in terms a Soroban developer needs.

### Decision rule from this case

A Soroban lint that overlaps with a Clippy lint is justified when:

1. The Soroban type has **non-standard cost semantics** (host crossings, metered allocations, storage I/O).
2. The Clippy lint's trigger condition is **orthogonal to cost** — it reasons about ownership, not about Soroban resource meters.
3. The Soroban lint's message is **actionable in Soroban terms** ("pass `Env` by value") rather than generic Rust advice ("remove the clone").

## Decision Checklist for New Lint Proposals

Before proposing a new lint, answer these questions:

- [ ] Is the pattern already caught by a Clippy lint? If yes, name it.
- [ ] Does the Soroban cost model make the pattern **more harmful** than in general Rust?
- [ ] Is the Clippy lint's trigger condition **orthogonal to cost**, or does it already capture the Soroban harm?
- [ ] If there is overlap, does the Soroban lint give the developer **actionable, Soroban-specific guidance** that Clippy cannot?

If the answer to the last three is yes, the lint belongs here. If not, it belongs in Clippy or in a follow-up issue upstream.
