# `address_from_string_in_loop`

**Default Severity:** `warn`

**Target Resource:** Host — repeated strkey parsing and address-handle construction.

## What it does

Flags calls to `soroban_sdk::Address::from_string` inside `for`, `while`, or `loop` bodies.

## Why is this bad?

`Address::from_string` parses and validates the strkey and constructs a host `Address`. Repeating that work for the same loop-invariant input pays the host cost on every iteration.

## Example

```rust
for item in items {
    let address = Address::from_string(&env, &strkey).unwrap();
    use_address(address, item);
}
```

Parse once before entering the loop:

```rust
let address = Address::from_string(&env, &strkey).unwrap();
for item in items {
    use_address(address.clone(), item);
}
```

The pass checks the resolved Soroban SDK definition, so unrelated functions with the same spelling (such as `String::from_str`) are not reported. Hoist only when the strkey is unchanged; otherwise suppress an intentional parse with `#[allow(address_from_string_in_loop)]`.
