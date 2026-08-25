#![no_std]
#![allow(unused)]

pub fn test_u128_suffices() {
    let x: u32 = 10;
    // Should fire: both operands are provably <= 64 bits (one is a cast from u32, one is a small literal)
    let y = (x as u128) + 5u128;

    // Should NOT fire: one operand is an unproven u128 parameter
    let z = y + 10u128;
}

pub fn token_balance_arithmetic(balance: i128) {
    // Should NOT fire: 'balance' is a caller-supplied i128, could exceed 64 bits
    let new_balance = balance - 100i128;
}
