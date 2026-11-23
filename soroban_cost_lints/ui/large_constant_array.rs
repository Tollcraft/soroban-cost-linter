#![allow(unused)]
#![warn(large_constant_array)]

const SMALL_ARRAY: [u8; 1000] = [0; 1000]; // Below threshold 4096

const LARGE_ARRAY: [u8; 5000] = [0; 5000]; // Above threshold

fn uses_array() {
    let _a = [0u8; 4000]; // Below threshold
    let _b = [0u8; 5000]; // Above threshold

    // Explicit array length
    let _c: [u32; 1200] = [0; 1200]; // 1200 * 4 = 4800 > 4096, Above threshold
    let _quiet: [u32; 1024] = [0; 1024]; // 1024 * 4 = 4096 bytes, stays quiet
}

#[allow(large_constant_array)]
fn suppressed() {
    let _d = [0u8; 5000]; // Should not lint
}

fn main() {}
