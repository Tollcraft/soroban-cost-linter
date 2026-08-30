// Test fixture for u128_where_u64_suffices

fn bad_counter_arithmetic() {
    let mut i: u128 = 0;
    i = i + 1; // Should Warn: literal/small counter addition
}

fn bad_derived_from_u32_len(len: u32) {
    let x: u128 = len as u128;
    let _y = x * 2; // Should Warn: derived from u32 length
}

fn good_token_balance(balance: u128, delta: u128) {
    let _new_balance = balance + delta; // Good: operates on genuine u128 token balance parameters
}

fn good_already_u64(a: u64, b: u64) {
    let _c = a * b; // Good: u64 arithmetic
}

fn main() {}
