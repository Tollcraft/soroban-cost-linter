# `signature_verification_in_loop`

**Default Severity:** `warn`

**Target Resource:** [CPU — elliptic-curve cryptographic host functions](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags calls to `env.crypto().ed25519_verify(...)`,
`env.crypto().secp256k1_recover(...)`, or `env.crypto().secp256r1_verify(...)`
when the call site sits inside a loop body (`for`, `while`, or `loop`).

## Why is this bad?

{% hint style="danger" %}
Elliptic-curve signature verification is one of the most CPU-expensive host
functions a Soroban contract can call — orders of magnitude more expensive
than a plain `WasmInsnExec` (see [Cost Rationale — CPU Instructions](../cost_rationale.md#1-cpu-instructions)).
Unlike a host call whose result is constant across iterations
(see [`unnecessary_host_function_call`](unnecessary_host_function_call.md)),
this cost cannot be hoisted out of the loop: each iteration is verifying a
*different* signature. Verifying signatures one at a time in a loop is
almost always a structural sign that the contract should be using a
signature scheme that supports batch or aggregate verification, or should
be delegating per-item authorization to a bulk entrypoint instead.
{% endhint %}

## Example

```rust
// ❌ Bad: one elliptic-curve check per iteration
for (public_key, message, signature) in submissions.iter() {
    env.crypto().ed25519_verify(&public_key, &message, &signature);
}
```

```rust
// ✅ Good: aggregate the submissions off-contract and verify a single
// aggregate signature (e.g. BLS), or accept only one signed batch payload
let aggregate_signature = /* combined signature covering the whole batch */;
env.crypto().ed25519_verify(&batch_public_key, &batch_message, &aggregate_signature);
```

## Suggested Fix

{% hint style="success" %}
Prefer a signature scheme that supports batch or aggregate verification
(e.g. BLS aggregate signatures) so the whole batch is checked with a single
host call. If that isn't available, consider whether per-item verification
can move to a bulk entrypoint on the caller side instead of repeating it once
per loop iteration on-chain.
{% endhint %}

## What is not reported

- Calls to `ed25519_verify`, `secp256k1_recover`, or `secp256r1_verify`
  outside of a loop body.
- Calls suppressed with `#[allow(signature_verification_in_loop)]`.

## Deliberately not covered

This lint starts from the three well-established Soroban SDK signature
primitives and leaves related patterns as documented follow-ups:

- **`Address::require_auth()` / `require_auth_for_args()`** — Soroban's
  higher-level authorization framework has its own cost profile and is
  tracked separately (see the `require_auth_in_loop` proposal).
- **BLS12-381 / BN254 pairing checks** (`bls12_381`, `bn254` modules) — these
  support genuine batch verification via `pairing_check`/multi-scalar
  multiplication, but their methods are general-purpose curve operations
  rather than a single named "verify" call, so recognizing a
  signature-verification *usage* of them reliably requires more context than
  a fixed method-name match.
