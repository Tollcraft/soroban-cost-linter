# `crypto_hash_of_constant`

**Default Severity:** `warn`

**Target Resource:** [CPU — cryptographic host functions](../cost_rationale.md#per-lint-resource-summary)

## What it does

Flags calls to `env.crypto().sha256(...)` or `env.crypto().keccak256(...)`
where the single argument passed to the hash is a compile-time constant: a
literal (`b"domain tag"`, `[1u8, 2, 3]`) or a `const` item.

## Why is this bad?

{% hint style="danger" %}
Cryptographic hashing is a metered host call, and among the more expensive
ones a Soroban contract can make. Hashing a value that is fixed at compile
time — a domain-separation tag, a fixed prefix, or a constant salt — pays
that full cost on every single invocation to produce a digest that never
changes between runs. The work is pure waste: the digest could be computed
once, offline, and embedded in the contract as a constant. Unlike a runtime
input whose hash genuinely depends on caller data, there is no behavioural
reason to recompute a constant digest at runtime.
{% endhint %}

This is a small, unambiguous win with no behavioural risk, and a complement to
[`signature_verification_in_loop`](signature_verification_in_loop.md): that
lint catches expensive crypto scaled by iteration count, while this one
catches expensive crypto that should not have run at all.

## Example

```rust
// ❌ Bad: the input is a compile-time constant, so the digest never changes
let domain_sep = b"my-contract";
let tagged = env.crypto().sha256(domain_sep); // re-hashes a fixed value every call
```

```rust
// ✅ Good: precompute the digest once and embed it as a constant
const DOMAIN_SEP_HASH: [u8; 32] = [
    /* bytes of sha256(b"my-contract"), computed offline */
];
// use DOMAIN_SEP_HASH directly; no host hash call, no per-invocation cost
```

## Suggested Fix

{% hint style="success" }
Compute the digest of the constant value once (e.g. with `sha256sum` or an
off-chain build step) and embed the resulting bytes as a `const`. Reference
that constant wherever the digest is needed instead of re-hashing the constant
at runtime. If the constant is a fixed prefix that is concatenated with
runtime data before hashing, only the runtime portion needs to be hashed.
{% endhint %}

## What is not reported

- Hashes whose argument is a runtime-derived value: a function parameter, a
  local variable, a loop variable, or any expression that depends on such a
  value.
- `CryptoHazmat` primitives (e.g. `secp256k1_recover`, `secp256r1_verify`),
  which are covered by other lints or are not general-purpose data hashes.
- Calls suppressed with `#[allow(crypto_hash_of_constant)]`.

## Deliberately not covered

This lint recognises only the two high-level `Crypto` hash methods the SDK
currently exposes by default — `sha256` and `keccak256`. It does **not**
cover:

- **`CryptoHazmat` hashes / permutations** (`poseidon_permutation`,
  `poseidon2_permutation`, etc.) — these take structured, multi-argument input
  and are application-specific, so a "hash of a constant" reading does not
  apply cleanly. If the SDK adds more high-level `Crypto` hash methods
  (e.g. `ripemd160`, `blake3`), they should be added to
  `CRYPTO_HASH_METHODS` in `soroban_cost_lints/src/lib.rs` and this page
  updated.
- **Constant-folding beyond literals and `const` items** — a value built from
  a literal via a constructor (e.g. `Bytes::from_slice(&env, b"prefix")`) is
  out of scope: the lint only treats the direct argument as constant when it
  is a literal or a `const` item, so such cases are intentionally left
  unreported rather than risked as false positives.
