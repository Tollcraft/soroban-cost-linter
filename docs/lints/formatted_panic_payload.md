# `formatted_panic_payload`

**Default Severity:** `warn`

**Target Resource:** [CPU instructions on the failure path, and WASM binary size on every deploy](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags three call shapes that pull Rust's `core::fmt` string-formatting
machinery into a `#![no_std]` Soroban contract:

1. Any `format!(...)` invocation.
2. `panic!(...)` when the macro call has at least one formatting argument
   (e.g. `panic!("balance {} below {}", a, b)`). A zero-argument
   `panic!("plain literal")` is **not** flagged — it never touches
   `core::fmt`.
3. `.expect(&format!(...))` — an `.expect()` call whose message argument is a
   `format!(...)` call. A plain `.expect("literal")` is **not** flagged.

## Why is this bad?

{% hint style="danger" %}
`format!` and a formatted `panic!` message both expand through
`core::fmt::Arguments` and the associated `Display`/`Debug` formatting code
paths. In a `#![no_std]` contract that machinery is not "free" the way it is
in a hosted Rust program: pulling it in inflates the compiled WASM binary —
a cost paid on **every deploy**, not just when the failure path runs — and
running it also spends CPU instructions formatting the message on the
failure path itself.
{% endhint %}

`panic_with_error!` combined with a `#[contracterror]` enum, by contrast,
compiles down to returning a plain integer error code. It carries neither
cost: no formatting machinery is linked in, and raising it is a single
constant-time host call.

Contract authors reach for `format!`/`panic!("...{}...")` reflexively
because it's the idiomatic, "free" thing to do everywhere else in Rust. That
habit is exactly what makes this a high-frequency, low-awareness cost
mistake in a metered `no_std` context.

## Example

```rust
// ❌ Bad: pulls in core::fmt for a message that's only useful in a debugger
fn bad(env: &Env, balance: i128, amount: i128) {
    if balance < amount {
        panic!("balance {} below required {}", balance, amount);
    }
}
```

```rust
// ✅ Good: a plain integer error code, no formatting machinery at all
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    InsufficientBalance = 1,
}

fn good(env: &Env, balance: i128, amount: i128) {
    if balance < amount {
        panic_with_error!(env, Error::InsufficientBalance);
    }
}
```

## Suggested Fix

{% hint style="success" %}
Replace the formatted message with a `#[contracterror]` enum variant and
raise it via `panic_with_error!(env, Error::Variant)`. If the formatted
detail is only useful for local debugging, gate it behind `#[cfg(test)]` or
strip it entirely before shipping — this lint does not fire in
`#[cfg(test)]` code (see below), so debug-only formatted panics in tests are
unaffected.
{% endhint %}

## What is not reported

- `panic!("plain literal")` — zero-argument `panic!`, no formatting
  machinery involved.
- `.expect("plain literal")` — a message that is not a `format!(...)` call.
- `.expect(msg)` where `msg` is a local variable, even if it happens to hold
  a `String` built elsewhere — this lint only follows the message argument
  one hop, to keep the pattern precise and avoid chasing values across a
  function.
- Any of the three shapes above when the enclosing item is under
  `#[cfg(test)]` — either directly on the function, or on an enclosing
  `mod tests { .. }`.
- Calls suppressed with `#[allow(formatted_panic_payload)]`.

## Test-vs-contract code: the `#[cfg(test)]` signal

The issue behind this lint asks for a way to tell contract code apart from
test code, and specifically calls out both `#[cfg(test)]` and
`#[contractimpl]` as candidate signals. This lint uses **`#[cfg(test)]`
only** (via `clippy_utils::is_in_test`, which covers both a `#[test]`
function and any parent marked `#[cfg(test)]`, including an enclosing
`mod tests { .. }`).

A `#[contractimpl]`-reachability analysis — proving an expression can only
be reached from a contract's public entrypoints — was considered and
rejected for a first version: it would require whole-crate call-graph
reasoning that is both fragile (indirect calls, trait dispatch, and
re-exports would need to be modeled or conservatively over-approximated)
and invasive relative to the rest of this crate's lints, which stay
single-function or bounded-depth. Staying with `#[cfg(test)]` keeps the
lint's behavior easy to reason about and consistent with the project's
stated preference for conservative, false-positive-averse checks.

One consequence: this lint fires on `format!`/formatted `panic!`/`expect`
usage in *any* non-test code compiled into the crate being checked, not
only code reachable from a `#[contractimpl]` entrypoint. In practice this is
rarely a problem because `cargo-cost-lint` is run against Soroban contract
crates, whose non-test code is overwhelmingly on or reachable from the
contract's execution paths.
