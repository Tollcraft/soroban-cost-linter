# `crypto_hash_of_loop_invariant`

**Default Severity:** `warn`

**Target Resource:** [CPU — cryptographic host functions](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags `sha256` and `keccak256` calls on `env.crypto()` inside a loop when the
hashed argument is a local binding declared outside the loop that the loop does
not mutate.

## Why is this bad?

{% hint style="danger" %}
Cryptographic hashes are metered host functions. If the input is the same on every iteration, the same digest is recomputed over and over, multiplying the cost by the trip count for a result that never changes. Hoisting the hash out of the loop computes it once and leaves the loop doing only the work that actually varies.
{% endhint %}

## Example

❌ Bad:

```rust
let payload = Bytes::from_array(&env, b"fixed payload");
for item in items {
    let digest = env.crypto().sha256(&payload);
    // ...digest is the same every iteration
}
```

✅ Good:

```rust
let payload = Bytes::from_array(&env, b"fixed payload");
let digest = env.crypto().sha256(&payload);
for item in items {
    // ...use digest
}
```

## Suggested Fix

{% hint style="success" %}
Bind the digest to a local before the loop and reuse it inside. Only hash inside the loop when the input genuinely changes from iteration to iteration.
{% endhint %}

## What is not reported

- An input derived from the loop variable, or declared inside the loop body.
- An input mutated anywhere in the loop.
- Literals and `const` items: those belong to [`crypto_hash_of_constant`](crypto_hash_of_constant.md), so the two lints never double-report the same call.
- Arbitrary expressions whose invariance cannot be proven; the pass only follows a plain local path (optionally behind `&`).

## Known limitations

The invariance analysis is deliberately conservative. When mutation analysis cannot give a verdict, the call is treated as loop-dependent and stays silent rather than risking a false positive.
