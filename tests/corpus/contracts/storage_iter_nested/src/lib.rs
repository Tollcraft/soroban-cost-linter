#![no_std]
use soroban_sdk::{symbol_short, Env, Symbol};

/// Storage key used in all test functions.
const DATA: Symbol = symbol_short!("data");
const DATA_I: Symbol = symbol_short!("data_i");
const DATA_J: Symbol = symbol_short!("data_j");

/// Test fixtures for the `soroban_storage_in_loop` lint covering nested loop patterns.
///
/// This module provides positive (should warn) and negative (should not warn)
/// test cases for storage operations inside nested loops. The lint detects
/// storage reads/writes in any loop body, including nested loops.
///
/// # Patterns Tested
/// - Nested `for` loops with storage operations
/// - Mixed loop types (for/while/loop) with storage
/// - Storage operations hoisted out of nested loops (good pattern)
/// - Loop-invariant storage access in nested loops
/// - Closures inside nested loops with storage access
///
/// # Lint Behavior
/// The `soroban_storage_in_loop` lint triggers on any storage operation
/// (get, set, has, extend_ttl) that occurs inside a loop body. For nested
/// loops, the operation is flagged if it appears in any level of nesting.
///
/// # Performance Impact
/// Storage operations in nested loops are particularly costly because they
/// execute O(n*m) times where n and m are the loop bounds. Each storage
/// operation crosses the Wasm-host boundary and incurs significant CPU cost.

// =======================================================================
// Positive test cases (should trigger soroban_storage_in_loop)
// =======================================================================

/// Nested for loops with storage write in inner loop.
///
/// This is the classic anti-pattern: a storage write executes on every
/// iteration of the inner loop. For bounds of 5x5, this results in 25
/// storage writes instead of 1.
pub fn bad_nested_write(env: Env) {
    for i in 0..5 {
        for j in 0..5 {
            env.storage().instance().set(&DATA, &(i + j));
        }
    }
}

/// Nested for loops with storage read in inner loop.
///
/// Storage reads in loops are equally problematic. Each read incurs the
/// host call overhead and ledger I/O cost.
pub fn bad_nested_read(env: Env) {
    for i in 0..5 {
        for j in 0..5 {
            let _: Option<i32> = env.storage().instance().get(&DATA);
        }
    }
}

/// Nested for loops with storage has check in inner loop.
///
/// The `has` method also crosses the host boundary and should be hoisted.
pub fn bad_nested_has(env: Env) {
    for i in 0..5 {
        for j in 0..5 {
            if env.storage().instance().has(&DATA) {
                // Do something
            }
        }
    }
}

/// Three-level nested loops with storage write.
///
/// Exponentially worse: 5x5x5 = 125 storage writes.
pub fn bad_triple_nested(env: Env) {
    for i in 0..5 {
        for j in 0..5 {
            for k in 0..5 {
                env.storage().instance().set(&DATA, &(i + j + k));
            }
        }
    }
}

/// Mixed loop types: outer for, inner while with storage.
///
/// The lint detects storage in any loop construct (for, while, loop).
pub fn bad_mixed_for_while(env: Env) {
    for i in 0..3 {
        let mut j = 0;
        while j < 3 {
            env.storage().instance().set(&DATA, &(i + j));
            j += 1;
        }
    }
}

/// Mixed loop types: outer while, inner for with storage.
pub fn bad_mixed_while_for(env: Env) {
    let mut i = 0;
    while i < 3 {
        for j in 0..3 {
            env.storage().instance().set(&DATA, &(i + j));
        }
        i += 1;
    }
}

/// Loop inside loop with storage in outer loop body.
///
/// Even if storage is in the outer loop (but still inside a loop),
/// it's flagged because it executes repeatedly.
pub fn bad_storage_in_outer_loop(env: Env) {
    for i in 0..5 {
        env.storage().instance().set(&DATA_I, &i); // Warn: in outer loop
        for j in 0..5 {
            // Inner loop without storage - but outer has it
            let _ = j;
        }
    }
}

/// Loop with storage in inner loop using different keys per iteration.
///
/// Using loop-variant keys doesn't make the pattern acceptable - the
/// storage operation still executes per iteration.
pub fn bad_nested_variant_keys(env: Env) {
    for i in 0..5 {
        for j in 0..5 {
            let key = Symbol::new(&env, "key");
            env.storage().instance().set(&key, &(i * j));
        }
    }
}

/// Nested loops with persistent storage access.
///
/// Persistent storage in loops is equally costly and flagged by the lint.
pub fn bad_nested_persistent(env: Env) {
    for i in 0..3 {
        for j in 0..3 {
            env.storage().persistent().set(&DATA, &(i + j));
        }
    }
}

/// Nested loops with temporary storage access.
pub fn bad_nested_temporary(env: Env) {
    for i in 0..3 {
        for j in 0..3 {
            env.storage().temporary().set(&DATA, &(i + j));
        }
    }
}

/// Nested loops with extend_ttl call.
///
/// Extending TTL inside loops is redundant and expensive.
pub fn bad_nested_extend_ttl(env: Env) {
    for i in 0..3 {
        for j in 0..3 {
            env.storage().instance().extend_ttl(100, 200);
        }
    }
}

// =======================================================================
// Negative test cases (should NOT trigger soroban_storage_in_loop)
// =======================================================================

/// Nested loops with computation only, storage outside.
///
/// This is the recommended pattern: accumulate results in memory
/// during the nested loops, then perform a single storage write.
pub fn good_nested_compute_then_store(env: Env) {
    let mut sum = 0i128;
    for i in 0..5 {
        for j in 0..5 {
            sum += i + j;
        }
    }
    env.storage().instance().set(&DATA, &sum);
}

/// Nested loops with read hoisted outside.
///
/// If the storage read is loop-invariant, hoist it completely outside
/// the nested structure.
pub fn good_nested_read_hoisted(env: Env) {
    let value: Option<i32> = env.storage().instance().get(&DATA);
    for i in 0..5 {
        for j in 0..5 {
            // Use value without storage access
            let _ = i + j;
        }
    }
    // value used here if needed
}

/// Nested loops with multiple storage operations all hoisted.
pub fn good_nested_multiple_hoisted(env: Env) {
    let has_data = env.storage().instance().has(&DATA);
    let value: Option<i32> = env.storage().instance().get(&DATA);

    for i in 0..3 {
        for j in 0..3 {
            let _ = (has_data, value, i, j);
        }
    }

    env.storage().instance().set(&DATA, &42);
}

/// Nested loops with storage in separate non-nested function.
///
/// Extracting storage operations to a helper function called once
/// outside the loops is a valid refactoring.
fn read_config(env: &Env) -> i32 {
    env.storage().instance().get(&DATA).unwrap_or(0)
}

pub fn good_nested_storage_in_helper(env: Env) {
    let config = read_config(&env);
    for i in 0..5 {
        for j in 0..5 {
            let _ = config + i + j;
        }
    }
}

/// Deeply nested loops with storage completely outside.
pub fn good_deep_nested_storage_outside(env: Env) {
    let mut result = 0i128;
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                result += i + j + k;
            }
        }
    }
    env.storage().instance().set(&DATA, &result);
}

/// Nested loops where inner loop has no storage, outer has one write.
///
/// A single write in the outer loop (not per inner iteration) is
/// acceptable and won't be flagged as nested loop storage.
pub fn good_single_write_in_outer(env: Env) {
    for i in 0..5 {
        for j in 0..5 {
            // No storage here
            let _ = j;
        }
        // One write per outer iteration - not in nested loop body
        env.storage().instance().set(&DATA_I, &i);
    }
}

/// Nested loops with storage only after all loops complete.
pub fn good_storage_after_all_loops(env: Env) {
    let mut results = soroban_sdk::Vec::new(&env);
    for i in 0..3 {
        for j in 0..3 {
            results.push_back(i + j);
        }
    }
    env.storage().instance().set(&DATA, &results);
}

/// Nested loops with conditional storage outside inner loop.
pub fn good_conditional_storage_outer(env: Env) {
    let mut should_write = false;
    for i in 0..3 {
        for j in 0..3 {
            if i == 1 && j == 1 {
                should_write = true;
            }
        }
    }
    if should_write {
        env.storage().instance().set(&DATA, &1);
    }
}

/// Nested closures (for_each) with storage - should warn if in loop context.
/// This tests the closure detection in nested scenarios.
pub fn bad_nested_closures_with_storage(env: Env) {
    for i in 0..3 {
        (0..3).for_each(|j| {
            env.storage().instance().set(&DATA, &(i + j));
        });
    }
}

/// Good: nested closures without storage, storage outside.
pub fn good_nested_closures_compute_only(env: Env) {
    let mut sum = 0i128;
    for i in 0..3 {
        (0..3).for_each(|j| {
            sum += i + j;
        });
    }
    env.storage().instance().set(&DATA, &sum);
}

/// Edge case: empty inner loop body with storage in outer.
/// The outer loop storage is not "nested" but still in a loop.
pub fn edge_outer_loop_only(env: Env) {
    for i in 0..5 {
        // Empty inner loop
        for _ in 0..0 {}
        env.storage().instance().set(&DATA_I, &i); // This is in a loop but not nested
    }
}

/// Edge case: loop with single iteration (bound = 1).
/// Still flagged because the pattern is the same.
pub fn edge_single_iteration_nested(env: Env) {
    for _ in 0..1 {
        for _ in 0..1 {
            env.storage().instance().set(&DATA, &1);
        }
    }
}

/// Good: Single iteration loops with storage outside.
pub fn good_single_iteration_outside(env: Env) {
    for _ in 0..1 {
        for _ in 0..1 {
            // No storage
        }
    }
    env.storage().instance().set(&DATA, &1);
}