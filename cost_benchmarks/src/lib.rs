//! Cost measurement harness for soroban-cost-linter lint reference pages.
//!
//! Each test below reproduces a specific anti-pattern and its suggested fix on
//! a minimal example, then prints the `env.cost_estimate().budget()` deltas so
//! the figures can be copied into the lint documentation.
//!
//! Run with:
//!   cd cost_benchmarks && cargo test -- --nocapture
//! (or from repo root: `cargo test --manifest-path cost_benchmarks/Cargo.toml -- --nocapture`)
//!
//! The output for each test includes:
//!   - CPU instruction count delta
//!   - Memory byte count delta
//!
//! All measurements use `Env::default()` (a local test-only environment).
//!
//! # Measurement caveats
//!
//! `Env::default()` counts *host* work only. Compiler-optimized-away Rust
//! loops are invisible to it, so the "good" halves wrap their results in
//! [`std::hint::black_box`] where the loop body is pure computation — without
//! that, `sum += seq` collapses into a closed form and the loop vanishes from
//! the numbers entirely.
//!
//! # Output contract
//!
//! Every test prints lines of the form
//! `lint_name | bad|good | scenario | cpu: N  mem: M`, which downstream
//! tooling parses by splitting on `|`. Keep the format stable.

#[cfg(test)]
use soroban_sdk::{symbol_short, Bytes, Env, Symbol};

#[soroban_sdk::contract]
pub struct DummyContract;
#[soroban_sdk::contractimpl]
impl DummyContract {}

/// Measurement scaffolding shared by the benchmark tests below.
///
/// Everything here is driven only by the `#[test]` benchmarks, so the module
/// is compiled solely under `cargo test`. This keeps the plain (non-test)
/// library build free of dead-code warnings and shrinks what a reviewer has
/// to read to reach the actual scenarios.
#[cfg(test)]
mod support {
    use super::DummyContract;
    pub(super) use soroban_sdk::Env;

    pub(super) const ITER_COUNT: u32 = 100;
    pub(super) const STORAGE_ITER_COUNT: u32 = 10;

    /// Host budget readings taken at a point in time.
    ///
    /// Groups the two counters we report so "before"/"after" snapshots stay in
    /// sync by construction instead of by two loosely-coupled local variables.
    #[derive(Clone, Copy)]
    pub(super) struct BudgetSnapshot {
        cpu_instructions: u64,
        memory_bytes: u64,
    }

    impl BudgetSnapshot {
        /// Reads the current cumulative budget counters from the environment.
        pub(super) fn capture(env: &Env) -> Self {
            let budget = env.cost_estimate().budget();
            Self {
                cpu_instructions: budget.cpu_instruction_cost(),
                memory_bytes: budget.memory_bytes_cost(),
            }
        }

        /// Host work consumed between `self` (before) and `after`.
        fn delta_since(self, after: Self) -> BudgetDelta {
            BudgetDelta {
                cpu_instructions: after.cpu_instructions - self.cpu_instructions,
                memory_bytes: after.memory_bytes - self.memory_bytes,
            }
        }
    }

    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Tolerance ratio applied when asserting that the anti-pattern (bad) variant
    /// is at least as expensive as the recommended (good) variant on cpu_instructions.
    /// A 5% tolerance avoids machine-flakiness or minor instruction count jitter.
    const DIRECTION_TOLERANCE_RATIO: f64 = 0.05;

    /// Difference between two [`BudgetSnapshot`]s, ready for printing.
    #[derive(Clone, Copy, Debug)]
    struct BudgetDelta {
        cpu_instructions: u64,
        memory_bytes: u64,
    }

    impl BudgetDelta {
        /// Emits one parseable result line; see the module docs for the format.
        fn report(&self, lint: &str, variant: &str, scenario: &str) {
            println!(
                "{:<25} | {:<4} | {:<34} | cpu: {}  mem: {}",
                lint, variant, scenario, self.cpu_instructions, self.memory_bytes
            );
        }
    }

    #[derive(Clone, Copy, Debug, Default)]
    struct BenchmarkPairRecord {
        bad: Option<BudgetDelta>,
        good: Option<BudgetDelta>,
    }

    static RECORDED_PAIRS: Mutex<Option<HashMap<String, BenchmarkPairRecord>>> = Mutex::new(None);

    fn record_and_verify(lint: &str, variant: &str, delta: BudgetDelta) {
        let pair_to_verify = {
            let mut guard = RECORDED_PAIRS.lock().unwrap();
            let map = guard.get_or_insert_with(HashMap::new);
            let entry = map.entry(lint.to_string()).or_default();
            match variant {
                "bad" => entry.bad = Some(delta),
                "good" => entry.good = Some(delta),
                _ => {}
            }
            if entry.bad.is_some() && entry.good.is_some() {
                let pair = map.remove(lint).unwrap();
                Some((pair.bad.unwrap(), pair.good.unwrap()))
            } else {
                None
            }
        };

        if let Some((bad, good)) = pair_to_verify {
            assert_cost_direction(lint, bad, good);
        }
    }

    fn assert_cost_direction(lint: &str, bad: BudgetDelta, good: BudgetDelta) {
        let min_expected_bad =
            (good.cpu_instructions as f64 * (1.0 - DIRECTION_TOLERANCE_RATIO)) as u64;
        assert!(
            bad.cpu_instructions >= min_expected_bad,
            "Cost direction assertion failed for lint '{}': anti-pattern variant (bad: {} cpu) is cheaper than recommended variant (good: {} cpu)",
            lint,
            bad.cpu_instructions,
            good.cpu_instructions
        );
    }

    /// Measures the host cost of `body` and prints it as one result line.
    ///
    /// Every benchmark follows the same three-step pattern: capture a
    /// snapshot, run the anti-pattern (or its fix), capture again and report
    /// the delta. Centralizing the pattern keeps the output aligned and makes
    /// it impossible to accidentally compare against a stale snapshot.
    pub(super) fn measure<F: FnOnce()>(
        env: &Env,
        lint: &str,
        variant: &str,
        scenario: &str,
        body: F,
    ) {
        let before = BudgetSnapshot::capture(env);
        body();
        let after = BudgetSnapshot::capture(env);
        let delta = before.delta_since(after);
        delta.report(lint, variant, scenario);
        record_and_verify(lint, variant, delta);
    }

    /// Registers [`DummyContract`] and runs `f` inside its storage context.
    ///
    /// Instance/persistent storage calls trap outside a contract invocation,
    /// so every storage-related benchmark wraps its body in this helper rather
    /// than repeating the registration and `as_contract` dance.
    pub(super) fn with_contract_storage<F: FnOnce()>(env: &Env, f: F) {
        let contract_id = env.register(DummyContract, ());
        env.as_contract(&contract_id, f);
    }

    /// Fills a native `Vec` with `count` copies of `element`.
    ///
    /// Pre-allocating `count` slots up front is the whole point of the
    /// "batched write" pattern: one amortized allocation instead of one host
    /// round-trip per element. `black_box` keeps the fill loop from being
    /// folded away, which would otherwise understate the CPU cost of the
    /// accumulation and flatter the comparison.
    pub(super) fn accumulate_range(count: u32, element: i32) -> std::vec::Vec<i32> {
        let mut values = std::vec::Vec::with_capacity(count as usize);
        for _ in 0..count {
            values.push(element);
            std::hint::black_box(&values);
        }
        values
    }
}

#[cfg(test)]
use support::*;

// ── symbol_new_for_short_literal ──────────────────────────────────────────

#[test]
fn bench_symbol_new_vs_short() {
    let env = Env::default();

    // ── Bad: Symbol::new (runtime host call) ──
    measure(&env, "symbol_new_for_short_literal", "bad", "Symbol::new in loop", || {
        for _ in 0..ITER_COUNT {
            let _sym = Symbol::new(&env, "hello");
        }
    });

    // ── Good: symbol_short! (compile-time, zero host cost) ──
    measure(&env, "symbol_new_for_short_literal", "good", "symbol_short! in loop", || {
        for _ in 0..ITER_COUNT {
            let _sym = symbol_short!("hello");
        }
    });
}

// ── redundant_env_clone ───────────────────────────────────────────────────

#[test]
fn bench_env_clone_vs_reuse() {
    let env = Env::default();

    // ── Bad: cloning Env per iteration ──
    measure(&env, "redundant_env_clone", "bad", "env.clone() in loop", || {
        for _ in 0..ITER_COUNT {
            let _cloned = env.clone();
        }
    });

    // ── Good: reuse env by reference ──
    measure(&env, "redundant_env_clone", "good", "&env in loop", || {
        for _ in 0..ITER_COUNT {
            let _env_ref = &env;
        }
    });
}

// ── unnecessary_host_function_call ─────────────────────────────────────────

#[test]
fn bench_host_fn_inside_vs_outside_loop() {
    let env = Env::default();

    // ── Bad: host function call every iteration (constant result) ──
    measure(&env, "unnecessary_host_fn_call", "bad", "ledger().sequence() in loop", || {
        for _ in 0..ITER_COUNT {
            let _seq = env.ledger().sequence();
        }
    });

    // ── Good: hoist host call outside the loop ──
    measure(&env, "unnecessary_host_fn_call", "good", "hoisted + reuse in loop", || {
        let seq = env.ledger().sequence();
        let mut sum: u64 = 0;
        for _ in 0..ITER_COUNT {
            sum += seq as u64;
        }
        // Prevent the compiler from optimizing away the loop body.
        std::hint::black_box(sum);
    });
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
    measure(&env, "bytes_append_in_loop", "bad", "Bytes::append in loop", || {
        let mut bytes = Bytes::new(&env);
        for _ in 0..ITER_COUNT {
            let chunk = Bytes::from_array(&env, &[1u8, 2, 3, 4]);
            bytes.append(&chunk);
        }
    });

    // ── Good: accumulate in native Vec, convert once ──
    measure(&env, "bytes_append_in_loop", "good", "native Vec + from_slice", || {
        let mut native: std::vec::Vec<u8> = std::vec::Vec::with_capacity((ITER_COUNT * 4) as usize);
        for _ in 0..ITER_COUNT {
            native.extend_from_slice(&[1u8, 2, 3, 4]);
        }
        let _bytes = Bytes::from_slice(&env, &native);
    });
}

// ── soroban_storage_in_loop ────────────────────────────────────────────────

#[test]
fn bench_storage_in_loop_vs_batch() {
    let env = Env::default();

    // ── Bad: one storage write per iteration ──
    with_contract_storage(&env, || {
        measure(&env, "soroban_storage_in_loop", "bad", "instance().set() in loop", || {
            for i in 0..STORAGE_ITER_COUNT {
                env.storage().instance().set(&i, &(i as i32));
            }
        });
    });

    // ── Good: accumulate in native Vec, write once ──
    with_contract_storage(&env, || {
        measure(&env, "soroban_storage_in_loop", "good", "accumulate + one write", || {
            let values = accumulate_range(STORAGE_ITER_COUNT, 0);
            env.storage()
                .instance()
                .set(&0u32, &(values.len() as i32));
        });
    });
}

// ── blind_storage_write / storage_write_without_read ───────────────────────

#[test]
fn bench_blind_storage_write() {
    let env = Env::default();

    with_contract_storage(&env, || {
        let key = symbol_short!("k");

        // ── Bad: blind write (no prior read) ──
        measure(&env, "blind_storage_write", "bad", "blind .set()", || {
            env.storage().instance().set(&key, &1i32);
        });

        // ── Good: read before write ──
        measure(&env, "blind_storage_write", "good", ".get() then .set()", || {
            let _ = env.storage().instance().get::<_, i32>(&key);
            env.storage().instance().set(&key, &2i32);
        });
    });
}

// ── discarded_storage_read ────────────────────────────────────────────────

#[test]
fn bench_discarded_storage_read() {
    let env = Env::default();

    with_contract_storage(&env, || {
        let key = symbol_short!("k2");
        env.storage().instance().set(&key, &42i32);

        // ── Bad: get() and discard ──
        measure(&env, "discarded_storage_read", "bad", "discarded .get()", || {
            let _ = env.storage().instance().get::<_, i32>(&key);
        });

        // ── Good: use has() ──
        measure(&env, "discarded_storage_read", "good", ".has()", || {
            let _ = env.storage().instance().has(&key);
        });
    });
}

// ── soroban_redundant_storage_read ─────────────────────────────────────────

#[test]
fn bench_redundant_storage_read() {
    let env = Env::default();

    with_contract_storage(&env, || {
        let key = symbol_short!("k3");
        env.storage().instance().set(&key, &42i32);

        // ── Bad: redundant gets ──
        measure(&env, "redundant_storage_read", "bad", "get() twice", || {
            let _a = env.storage().instance().get::<_, i32>(&key);
            let _b = env.storage().instance().get::<_, i32>(&key);
        });

        // ── Good: read once and reuse ──
        measure(&env, "redundant_storage_read", "good", "get() once + reuse", || {
            let a = env.storage().instance().get::<_, i32>(&key);
            let _b = a;
        });
    });
}

// ── instance_storage_for_unbounded_data ────────────────────────────────────

#[test]
fn bench_instance_storage_unbounded_data() {
    let env = Env::default();

    with_contract_storage(&env, || {
        let key = symbol_short!("vec");

        // ── Bad: Vec in instance storage ──
        measure(&env, "instance_unbounded_data", "bad", "Vec in instance storage", || {
            let mut participants: soroban_sdk::Vec<i32> = env
                .storage()
                .instance()
                .get(&key)
                .unwrap_or_else(|| soroban_sdk::Vec::new(&env));
            participants.push_back(1);
            env.storage().instance().set(&key, &participants);
        });

        // ── Good: item in persistent storage ──
        measure(&env, "instance_unbounded_data", "good", "entry in persistent", || {
            let participant_key = 1i32;
            env.storage().persistent().set(&participant_key, &true);
        });
    });
}

// ── loop_invariant_storage_access ──────────────────────────────────────────

#[test]
fn bench_loop_invariant_storage_access() {
    let env = Env::default();

    with_contract_storage(&env, || {
        let key = symbol_short!("k4");
        env.storage().instance().set(&key, &42i32);

        // ── Bad: get() inside loop ──
        measure(&env, "loop_invariant_storage", "bad", "get() inside loop", || {
            let mut sum = 0;
            for _ in 0..STORAGE_ITER_COUNT {
                let val: i32 = env.storage().instance().get(&key).unwrap();
                sum += val;
            }
            std::hint::black_box(sum);
        });

        // ── Good: get() outside loop ──
        measure(&env, "loop_invariant_storage", "good", "get() outside loop", || {
            let val: i32 = env.storage().instance().get(&key).unwrap();
            let mut sum2 = 0;
            for _ in 0..STORAGE_ITER_COUNT {
                sum2 += val;
            }
            std::hint::black_box(sum2);
        });
    });
}

// ── storage_key_construction_in_loop ───────────────────────────────────────

#[test]
fn bench_storage_key_construction_in_loop() {
    let env = Env::default();

    // ── Bad: build key in loop ──
    measure(&env, "storage_key_in_loop", "bad", "build key in loop", || {
        for _ in 0..ITER_COUNT {
            let key = symbol_short!("key");
            std::hint::black_box(key);
        }
    });

    // ── Good: build key outside loop ──
    measure(&env, "storage_key_in_loop", "good", "hoisted key", || {
        let key = symbol_short!("key");
        for _ in 0..ITER_COUNT {
            std::hint::black_box(&key);
        }
    });
}
