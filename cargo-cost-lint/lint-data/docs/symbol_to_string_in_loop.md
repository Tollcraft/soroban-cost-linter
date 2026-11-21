# `symbol_to_string_in_loop`

**Default Severity:** `warn`

**Target Resource:** [Memory — host string allocations](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags `Symbol::to_string()` calls inside a syntactic `for`, `while` or `loop`
body.

## Why is this bad?

{% hint style="danger" %}
`Symbol::to_string()` allocates a fresh host `String` on every call. Used in a loop to compare, log or rebuild a key, it allocates once per iteration and throws the result away. `Symbol` values can usually be compared directly, or the `String` can be built once before the loop and reused.
{% endhint %}

## Example

❌ Bad:

```rust
for candidate in candidates {
    if candidate.to_string() == selected {
        // ...
    }
}
```

✅ Good:

```rust
for candidate in candidates {
    if candidate == selected {
        // ...
    }
}
```

Or, when the text is genuinely needed:

```rust
let selected_text = selected.to_string();
for candidate in candidates {
    // ...compare against selected_text
}
```

## Suggested Fix

{% hint style="success" %}
Compare the `Symbol` values directly, or build the `String` once before the loop. No automatic rewrite is offered because the right fix depends on what the converted string is used for.
{% endhint %}

## What is not reported

- The same call outside a loop.
- `to_string` on a `soroban_sdk::String` or on a native local type such as `i32`.
- A conversion hoisted into a local before the loop.
- A conversion inside a closure that the loop does not call: the syntactic loop
  gate does not cross into closures.
- Calls suppressed with `#[allow(symbol_to_string_in_loop)]`.

## Known limitations

Only syntactic loops are considered. A helper function called from a loop is
not analysed in the caller's loop context.
