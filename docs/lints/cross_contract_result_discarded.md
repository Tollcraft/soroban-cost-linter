# `cross_contract_result_discarded`

**Default Severity:** `warn`

**Target Resource:** [CPU — host function dispatch and cross-contract VM instantiation](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags a `Env::invoke_contract(...)` call whose non-unit return value is discarded:

- bound to the wildcard pattern — `let _ = env.invoke_contract::<T>(...);`
- dropped as a bare statement — `env.invoke_contract::<T>(...);`

A call whose result is bound to a named variable, used as an argument, or whose
result type is the unit type `()` is not reported.

## Why is this bad?

{% hint style="danger" %}
A cross-contract invocation is among the most expensive operations a Soroban
contract can perform: the host instantiates a new VM context for the callee,
re-charges its own dispatch and instantiation overhead (`DispatchHostFunction`),
runs the callee's metered execution, and then converts the return value back
across the guest/host boundary. Calling one and throwing the result away pays
for all of that to learn nothing. Sometimes the call is made for its side effect
and the return value is genuinely uninteresting; often it is a bug where the
author meant to check a status and did not. See the
[Cost Rationale — What Dominates](../cost_rationale.md#what-dominates) for the
relative cost hierarchy.
{% endhint %}

## Example

```rust
// ❌ Bad: the return value (e.g. a status or balance) is thrown away
let _ = env.invoke_contract::<i128>(&addr, &symbol_short!("balance"), ());
env.invoke_contract::<()>(&addr, &symbol_short!("poke"), ()); // unit result — not flagged
```

```rust
// ✅ Good: the result is actually used
let balance: i128 = env.invoke_contract(&addr, &symbol_short!("balance"), ());
if balance < minimum {
    panic!("insufficient balance");
}
```

```rust
// ✅ Good (deliberate): the call is made for its side effect and the
// result is intentionally not needed — bind to a named variable to silence
let _result = env.invoke_contract::<()>(&addr, &symbol_short!("poke"), ());
```

## Suggested Fix

{% hint style="success" %}
If the return value matters, bind it to a named variable and use it (e.g. check
a status, branch on it, or store it). If the call is made purely for its side
effect and the return value is genuinely uninteresting, bind it to a named
variable such as `let _result = ...` or add
`#[allow(cross_contract_result_discarded)]` to suppress the warning. Note that
`let _ = ...` itself still triggers this lint — use a named binding rather than
the wildcard to signal the discard is deliberate.
{% endhint %}

## What is not reported

- Calls whose result is bound to a named variable (including one with an `_`
  prefix, e.g. `let _result = ...`).
- Calls whose result is consumed as an argument or otherwise used.
- Calls that return the unit type `()` (e.g. `invoke_contract::<()>(...)`).
- Calls suppressed with `#[allow(cross_contract_result_discarded)]`.

## Deliberately not covered

This lint intentionally stays a structural check rather than a general dead-code
analysis. It only considers the two clear cases — a wildcard `let _ =` binding
and a bare `;` statement — and does not attempt to decide whether a value bound
to a named variable is later used. Extending it into general unused-result
analysis would overlap with the compiler's own `unused_must_use` machinery and is
out of scope.
