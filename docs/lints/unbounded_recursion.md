# `unbounded_recursion`

**Default Severity:** `warn`

**Target Resource:** [CPU — call/stack metering and the guest memory cap](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags a function that participates in a recursive call cycle (direct recursion —
a function calling itself — or mutual recursion through two or more functions)
when the recursion depth is driven by caller-supplied input.

A Soroban contract runs against a fixed per-transaction resource budget. When the
caller decides how deep the recursion goes, they also decide how much CPU and
stack the transaction consumes. Deep enough, and the transaction exhausts its CPU
budget or the guest's memory cap and fails — after the caller has already paid to
get there. This is invisible in testing: a recursive function that terminates
comfortably on a three-element test input behaves very differently on a
three-thousand-element one, with no compile-time error to warn anyone.

The lint names the cycle it found, not just the function it was standing in —
`process -> process_child -> process` is actionable, `recursion detected` is not.

## Why is this bad?

{% hint style="danger" %}
Recursion whose depth is controlled by caller input is an unbounded-cost pattern
in the same family as [`unbounded_input_loop`](../cost_rationale.md#per-lint-resource-summary): the caller,
not the contract author, decides how much compute and stack the transaction
consumes. The deeper the recursion, the more `CallHostFunction`-adjacent call
overhead and stack the guest pays for, until the transaction blows its CPU budget
or memory cap. See the [Cost Rationale](../cost_rationale.md) for the relative
cost hierarchy.
{% endhint %}

## Example

```rust
// ❌ Bad: depth is the caller-supplied slice length
fn walk(items: &[u32]) {
    if items.is_empty() {
        return;
    }
    walk(&items[1..]);
}

// ❌ Bad: mutual recursion, depth still caller-driven
fn process(items: &[u32]) {
    if items.is_empty() {
        return;
    }
    process_child(items);
}
fn process_child(items: &[u32]) {
    process(&items[1..]);
}
```

```rust
// ✅ Good: rewrite as an iterative loop with an explicit, bounded bound
fn walk(items: &[u32]) {
    let mut rest = items;
    while !rest.is_empty() {
        // do work with rest[0]
        rest = &rest[1..];
    }
}

// ✅ Good: depth is a compile-time constant
fn countdown(n: u32) {
    if n == 0 {
        return;
    }
    countdown(n - 1);
}
```

## Suggested Fix

{% hint style="success" %}
Bound the depth (e.g. cap iterations at a constant) or convert the recursion to
an explicit loop / work-list. If the recursion is intentional and genuinely
bounded, allow it with `#[allow(unbounded_recursion)]`.
{% endhint %}

## The bounded-vs-unbounded rule

The lint reports a recursive cycle **only when it can positively prove the depth
is caller-controlled**. Concretely, a recursive call is treated as **unbounded**
when its argument is:

- a slicing / tail operation on caller data — `x[..]`, `x[1..]`, `&x[1..]`,
  `x.to_vec()` on a slice, `x.pop()`, `x.split_first()`, `x.split_off()`,
  `x.drain()`, `x.remove()`; or
- a caller-supplied collection (`Vec`, `String`, `&[T]`, `VecDeque`,
  `LinkedList`) passed with no structural progress.

A recursive cycle is considered **bounded (and stays silent)** when the
recursive call instead passes:

- a compile-time-constant argument — an integer/array literal, or arithmetic on
  constants (e.g. `f(3)`, `fixed_array([0, 0, 0])`); or
- a decrementing counter toward a constant base case, e.g. `countdown(n - 1)`
  with `if n == 0`.

Where the analysis **cannot prove a bound either way** — plain integer parameters
threaded through, generic recursion, complex conditionals — it **stays silent**
rather than risk a false positive. A lint that flags every recursive function
would be switched off immediately, so the default is always "do not report".

## Deliberately not covered

- Recursion through **trait objects, function pointers, or closures**. The call
  target cannot be resolved to a single local `DefId`, so it is never recorded as
  an edge and never forms a cycle the lint reports. This matches the existing
  cross-function lints, which give up on those deliberately.
- Recursion whose bound the analysis cannot determine. The walker's principle is
  explicit: when in doubt, stay silent.

## What is not reported

Everything listed under "bounded" above, plus any recursion the analysis cannot
classify. When in doubt, the lint does not fire.

## Known false positives / limitations

See [docs/false_positives.md](../../docs/false_positives.md) for the current list
of accepted limitations and known false-positive classes.
