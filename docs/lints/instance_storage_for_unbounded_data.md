# `instance_storage_for_unbounded_data`

**Default Severity:** `warn`

**Target Resource:** [Storage — ledger entry accesses and ledger I/O bytes](../cost_rationale.md#per-lint-resource-summary)

## What it does

Detects writes to `env.storage().instance()` where the value being written is
an unbounded Soroban SDK collection type — `Vec`, `Map`, or `Bytes`.

## Why is this bad?

{% hint style="danger" %}
Soroban's instance storage is loaded and rewritten as a **single blob on
every invocation** of the contract — not just the calls that touch the field
in question. Putting a growing collection in instance storage means every
future call, no matter what it does, pays to read and rewrite the entire
collection's current size. The fee climbs quietly over the life of the
contract and never shows up in a single-call test, because a fresh contract
starts with an empty (or small) collection and the cost only becomes visible
once real usage has grown it. See the
[Cost Rationale — Storage](../cost_rationale.md#3-storage-ledger-entry-accesses-and-ledger-io)
for how ledger entry accesses and I/O bytes are charged.

Persistent storage, keyed per entry, is the structurally correct shape for
data whose size grows over time — it is not covered further here.
{% endhint %}

## Example

```rust
// ❌ Bad: a Vec living in instance storage is re-read and re-written, in
//          full, on every single contract call — including calls that
//          have nothing to do with this field.
fn record_participant(env: Env, participant: Address) {
    let mut participants: Vec<Address> =
        env.storage().instance().get(&PARTICIPANTS).unwrap_or(Vec::new(&env));
    participants.push_back(participant);
    env.storage().instance().set(&PARTICIPANTS, &participants);
}
```

```rust
// ✅ Good: persistent storage keys each participant as its own entry, so
//          the per-invocation cost stays constant regardless of how many
//          participants have accumulated.
fn record_participant(env: Env, participant: Address) {
    env.storage().persistent().set(&participant, &true);
}
```

## Suggested Fix

{% hint style="success" %}
Use persistent storage keyed per entry instead of accumulating growing data
under a single instance-storage key.
{% endhint %}

## Where the bounded/unbounded line is drawn

This lint is deliberately conservative — it only fires when the value
expression passed to `.instance().set(key, value)` has a type that resolves
*directly* to `soroban_sdk::Vec`, `soroban_sdk::Map`, or `soroban_sdk::Bytes`:

- **Flagged:** the value's own type is one of the three SDK container types.
  A value that is itself an SDK collection is unambiguously unbounded —
  there is no interpretation under which storing it in a per-invocation blob
  is safe once the contract sees real usage.
- **Not flagged — scalars and fixed-size values:** `u32`, `i64`, `bool`,
  `Address`, `[u8; 32]`, and similar statically-sized types never resolve to
  a container ADT, so they are never flagged. Their size in the instance
  blob is fixed for the life of the contract.
- **Not flagged — configuration-shaped structs:** a plain struct (e.g. an
  admin/config record with a handful of scalar fields) resolves to its own
  ADT, not to `Vec`/`Map`/`Bytes`, so it is not flagged even though it lives
  in instance storage — that is exactly the pattern instance storage is for.
- **Not flagged — a collection nested inside a struct field:** if a
  user-defined struct or enum merely *contains* a `Vec`/`Map`/`Bytes` field,
  the value passed to `.set()` resolves to the wrapping struct's ADT, not to
  the container type, so the lint does not look inside it. Recognizing that
  case reliably would require whole-program reasoning about which struct
  fields can grow unboundedly, which this lint does not attempt — the
  tradeoff is intentional: it accepts missing that pattern in exchange for
  never flagging a false positive on an unrelated struct field.

## What is not reported

- Writes to `persistent()` or `temporary()` storage — those stores are keyed
  per entry, so an unbounded value there is the expected, correct shape and
  is out of scope for this lint.
- Reads (`get`, `has`) on instance storage — only the write (`set`) is
  checked, since that is the operation that grows the stored blob.
- Scalar values, fixed-size arrays, and plain structs written to instance
  storage, as described above.
- A `Vec`/`Map`/`Bytes` value nested inside a struct or enum field, rather
  than passed directly as the value argument.
- Calls suppressed with `#[allow(instance_storage_for_unbounded_data)]`.
