//! Cost measurement harness for soroban-cost-linter lint reference pages.
//!
//! Each test below reproduces a specific anti-pattern and its suggested fix on
//! a minimal example, then prints the env.budget() deltas so the figures can
//! be copied into the lint documentation.
//!
//! Run with:
//!   cargo test -- --nocapture
//!
//! The output for each test includes:
//!   - CPU instruction count delta
//!   - Memory byte count delta
//!
//! All measurements use `Env::default()` (a local test-only environment).

use soroban_sdk::{symbol_short, Bytes, Env, Symbol};

const ITER_COUNT: u32 = 100;
const STORAGE_ITER_COUNT: u32 = 10;

// ── symbol_new_for_short_literal ──────────────────────────────────────────

#[test]
fn bench_symbol_new_vs_short() {
    let env = Env::default();

    // ── Bad: Symbol::new (runtime host call) ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    for _ in 0..ITER_COUNT {
        let _sym = Symbol::new(&env, "hello");
    }
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "symbol_new_for_short_literal | bad  | Symbol::new x{}   | cpu: {}  mem: {}",
        ITER_COUNT,
        cpu_after - cpu_before,
        mem_after - mem_before
    );

    // ── Good: symbol_short! (compile-time, zero host cost) ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    for _ in 0..ITER_COUNT {
        let _sym = symbol_short!("hello");
    }
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "symbol_new_for_short_literal | good | symbol_short! x{} | cpu: {}  mem: {}",
        ITER_COUNT,
        cpu_after - cpu_before,
        mem_after - mem_before
    );
}

// ── redundant_env_clone ───────────────────────────────────────────────────

#[test]
fn bench_env_clone_vs_reuse() {
    let env = Env::default();

    // ── Bad: cloning Env per iteration ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    for _ in 0..ITER_COUNT {
        let _cloned = env.clone();
    }
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "redundant_env_clone       | bad  | env.clone() x{}   | cpu: {}  mem: {}",
        ITER_COUNT,
        cpu_after - cpu_before,
        mem_after - mem_before
    );

    // ── Good: reuse env by reference ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    for _ in 0..ITER_COUNT {
        let _env_ref = &env;
    }
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "redundant_env_clone       | good | &env x{}          | cpu: {}  mem: {}",
        ITER_COUNT,
        cpu_after - cpu_before,
        mem_after - mem_before
    );
}

// ── unnecessary_host_function_call ─────────────────────────────────────────

#[test]
fn bench_host_fn_inside_vs_outside_loop() {
    let env = Env::default();

    // ── Bad: host function call every iteration (constant result) ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    for _ in 0..ITER_COUNT {
        let _seq = env.ledger().sequence();
    }
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "unnecessary_host_fn_call  | bad  | ledger().sequence() in loop x{} | cpu: {}  mem: {}",
        ITER_COUNT,
        cpu_after - cpu_before,
        mem_after - mem_before
    );

    // ── Good: hoist host call outside the loop ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    let seq = env.ledger().sequence();
    let mut sum: u64 = 0;
    for _ in 0..ITER_COUNT {
        sum += seq as u64;
    }
    // Prevent the compiler from optimizing away the loop body.
    std::hint::black_box(sum);
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "unnecessary_host_fn_call  | good | hoisted + reuse x{}             | cpu: {}  mem: {}",
        ITER_COUNT,
        cpu_after - cpu_before,
        mem_after - mem_before
    );
}

// ── bytes_append_in_loop ───────────────────────────────────────────────────

#[test]
fn bench_bytes_append_in_loop_vs_batch() {
    let env = Env::default();

    // ── Bad: appending to SDK Bytes per iteration.
    // NOTE: each iteration creates a fresh Bytes chunk AND appends it —
    // two host calls per loop body. A real-world loop might have the
    // chunks pre-existing (e.g., from storage), in which case only the
    // `append` call is host work and the per-iteration cost is lower.
    // The O(n²) growth behaviour of the container buffer dominates
    // regardless.
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    let mut bytes = Bytes::new(&env);
    for _ in 0..ITER_COUNT {
        let chunk = Bytes::from_array(&env, &[1u8, 2, 3, 4]);
        bytes.append(&chunk);
    }
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "bytes_append_in_loop      | bad  | Bytes::append in loop x{}  | cpu: {}  mem: {}",
        ITER_COUNT,
        cpu_after - cpu_before,
        mem_after - mem_before
    );

    // ── Good: accumulate in native Vec, convert once ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    let mut native: std::vec::Vec<u8> =
        std::vec::Vec::with_capacity((ITER_COUNT * 4) as usize);
    for _ in 0..ITER_COUNT {
        native.extend_from_slice(&[1u8, 2, 3, 4]);
    }
    let _bytes = Bytes::from_slice(&env, &native);
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "bytes_append_in_loop      | good | native Vec + from_slice x{} | cpu: {}  mem: {}",
        ITER_COUNT,
        cpu_after - cpu_before,
        mem_after - mem_before
    );
}

// ── soroban_storage_in_loop ────────────────────────────────────────────────

#[test]
fn bench_storage_in_loop_vs_batch() {
    let env = Env::default();

    // ── Bad: one storage write per iteration ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    for i in 0..STORAGE_ITER_COUNT {
        env.storage().instance().set(&i, &(i as i32));
    }
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "soroban_storage_in_loop   | bad  | instance().set() in loop x{} | cpu: {}  mem: {}",
        STORAGE_ITER_COUNT,
        cpu_after - cpu_before,
        mem_after - mem_before
    );

    // ── Good: accumulate in native Vec, write once ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    let mut values: std::vec::Vec<i32> =
        std::vec::Vec::with_capacity(STORAGE_ITER_COUNT as usize);
    for i in 0..STORAGE_ITER_COUNT {
        values.push(i as i32);
    }
    env.storage()
        .instance()
        .set(&0u32, &(values.len() as i32));
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "soroban_storage_in_loop   | good | accumulate + one write  x{} | cpu: {}  mem: {}",
        STORAGE_ITER_COUNT,
        cpu_after - cpu_before,
        mem_after - mem_before
    );
}

// ── blind_storage_write / storage_write_without_read ───────────────────────

#[test]
fn bench_blind_storage_write() {
    let env = Env::default();
    let key = symbol_short!("k");
    
    // ── Bad: blind write (no prior read) ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    env.storage().instance().set(&key, &1i32);
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "blind_storage_write       | bad  | blind .set()              | cpu: {}  mem: {}",
        cpu_after - cpu_before,
        mem_after - mem_before
    );

    // ── Good: read before write ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    let _ = env.storage().instance().get::<_, i32>(&key);
    env.storage().instance().set(&key, &2i32);
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "blind_storage_write       | good | .get() then .set()        | cpu: {}  mem: {}",
        cpu_after - cpu_before,
        mem_after - mem_before
    );
}

// ── discarded_storage_read ────────────────────────────────────────────────

#[test]
fn bench_discarded_storage_read() {
    let env = Env::default();
    let key = symbol_short!("k2");
    env.storage().instance().set(&key, &42i32);

    // ── Bad: get() and discard ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    let _ = env.storage().instance().get::<_, i32>(&key);
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "discarded_storage_read    | bad  | discarded .get()          | cpu: {}  mem: {}",
        cpu_after - cpu_before,
        mem_after - mem_before
    );

    // ── Good: use has() ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    let _ = env.storage().instance().has(&key);
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "discarded_storage_read    | good | .has()                    | cpu: {}  mem: {}",
        cpu_after - cpu_before,
        mem_after - mem_before
    );
}

// ── soroban_redundant_storage_read ─────────────────────────────────────────

#[test]
fn bench_redundant_storage_read() {
    let env = Env::default();
    let key = symbol_short!("k3");
    env.storage().instance().set(&key, &42i32);

    // ── Bad: redundant gets ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    let _a = env.storage().instance().get::<_, i32>(&key);
    let _b = env.storage().instance().get::<_, i32>(&key);
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "redundant_storage_read    | bad  | get() twice               | cpu: {}  mem: {}",
        cpu_after - cpu_before,
        mem_after - mem_before
    );

    // ── Good: read once and reuse ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    let a = env.storage().instance().get::<_, i32>(&key);
    let _b = a;
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "redundant_storage_read    | good | get() once + reuse        | cpu: {}  mem: {}",
        cpu_after - cpu_before,
        mem_after - mem_before
    );
}

// ── instance_storage_for_unbounded_data ────────────────────────────────────

#[test]
fn bench_instance_storage_unbounded_data() {
    let env = Env::default();
    let key = symbol_short!("vec");
    
    // ── Bad: Vec in instance storage ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    let mut participants: soroban_sdk::Vec<i32> = env.storage().instance().get(&key).unwrap_or(soroban_sdk::Vec::new(&env));
    participants.push_back(1);
    env.storage().instance().set(&key, &participants);
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "instance_unbounded_data   | bad  | Vec in instance storage   | cpu: {}  mem: {}",
        cpu_after - cpu_before,
        mem_after - mem_before
    );

    // ── Good: item in persistent storage ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    let participant_key = 1i32;
    env.storage().persistent().set(&participant_key, &true);
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "instance_unbounded_data   | good | entry in persistent       | cpu: {}  mem: {}",
        cpu_after - cpu_before,
        mem_after - mem_before
    );
}

// ── loop_invariant_storage_access ──────────────────────────────────────────

#[test]
fn bench_loop_invariant_storage_access() {
    let env = Env::default();
    let key = symbol_short!("k4");
    env.storage().instance().set(&key, &42i32);

    // ── Bad: get() inside loop ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    let mut sum = 0;
    for _ in 0..STORAGE_ITER_COUNT {
        let val: i32 = env.storage().instance().get(&key).unwrap();
        sum += val;
    }
    std::hint::black_box(sum);
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "loop_invariant_storage    | bad  | get() inside loop x{}      | cpu: {}  mem: {}",
        STORAGE_ITER_COUNT,
        cpu_after - cpu_before,
        mem_after - mem_before
    );

    // ── Good: get() outside loop ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    let val: i32 = env.storage().instance().get(&key).unwrap();
    let mut sum2 = 0;
    for _ in 0..STORAGE_ITER_COUNT {
        sum2 += val;
    }
    std::hint::black_box(sum2);
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "loop_invariant_storage    | good | get() outside loop x{}     | cpu: {}  mem: {}",
        STORAGE_ITER_COUNT,
        cpu_after - cpu_before,
        mem_after - mem_before
    );
}

// ── storage_key_construction_in_loop ───────────────────────────────────────

#[test]
fn bench_storage_key_construction_in_loop() {
    let env = Env::default();

    // ── Bad: build key in loop ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    for _ in 0..ITER_COUNT {
        let key = symbol_short!("key");
        std::hint::black_box(key);
    }
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "storage_key_in_loop       | bad  | build key in loop x{}     | cpu: {}  mem: {}",
        ITER_COUNT,
        cpu_after - cpu_before,
        mem_after - mem_before
    );

    // ── Good: build key outside loop ──
    let cpu_before = env.budget().cpu_insn_count();
    let mem_before = env.budget().mem_bytes_count();
    let key = symbol_short!("key");
    for _ in 0..ITER_COUNT {
        std::hint::black_box(&key);
    }
    let cpu_after = env.budget().cpu_insn_count();
    let mem_after = env.budget().mem_bytes_count();
    println!(
        "storage_key_in_loop       | good | hoisted key x{}           | cpu: {}  mem: {}",
        ITER_COUNT,
        cpu_after - cpu_before,
        mem_after - mem_before
    );
}
