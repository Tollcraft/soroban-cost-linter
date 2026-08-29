pub mod soroban_sdk {
    pub mod vec {
        pub struct Vec<T>(pub std::vec::Vec<T>);
        impl<T> Vec<T> {
            pub fn contains(&self, _item: &T) -> bool { false }
            pub fn position(&self, _f: impl FnMut(&T) -> bool) -> Option<usize> { None }
        }
    }

    pub mod map {
        pub struct Map<K, V>(std::marker::PhantomData<(K, V)>);
        impl<K, V> Map<K, V> {
            pub fn contains_key(&self, _k: &K) -> bool { false }
        }
    }
}

use soroban_sdk::vec::Vec as SorobanVec;

// =======================================================================
// linear_scan_in_loop — Fixtures
// =======================================================================

// Firing: the scanned value is loop-invariant, so the O(n) scan is genuinely
// repeated every iteration.
fn bad_scan_in_for_loop() {
    let v: SorobanVec<i32> = SorobanVec(std::vec::Vec::new());
    let target = 5;
    for _ in 0..10 {
        let _ = v.contains(&target); // Should Warn
    }
}

// Firing: while loop shape.
fn bad_scan_in_while_loop() {
    let v: SorobanVec<i32> = SorobanVec(std::vec::Vec::new());
    let target = 5;
    let mut i = 0;
    while i < 10 {
        let _ = v.contains(&target); // Should Warn
        i += 1;
    }
}

// Firing: iterator closure shape.
fn bad_scan_in_iterator_closure() {
    let v: SorobanVec<i32> = SorobanVec(std::vec::Vec::new());
    let target = 5;
    (0..3).for_each(|_| {
        let _ = v.contains(&target); // Should Warn
    });
}

// Near-miss: the scanned value is the loop variable, so the scan is genuine
// per-iteration work that cannot be hoisted.
fn near_miss_scan_depends_on_loop_state() {
    let v: SorobanVec<i32> = SorobanVec(std::vec::Vec::new());
    for x in 0..10 {
        let _ = v.contains(&x); // Should not warn
    }
}

// Near-miss: the argument is an impure expression (a method call), so we
// conservatively assume it may vary per iteration.
fn near_miss_scan_impure_argument() {
    let v: SorobanVec<i32> = SorobanVec(std::vec::Vec::new());
    let wrapper = (5,);
    for _ in 0..10 {
        let _ = v.contains(&wrapper.0.clone()); // Should not warn
    }
}

// Firing: position with a loop-invariant closure body.
fn bad_position_in_loop() {
    let v: SorobanVec<i32> = SorobanVec(std::vec::Vec::new());
    let target = 5;
    for _ in 0..10 {
        let _ = v.position(|x| *x == target); // Should Warn
    }
}

fn good_scan_outside_loop() {
    let v: SorobanVec<i32> = SorobanVec(std::vec::Vec::new());
    let target = 5;
    let _ = v.contains(&target); // Good — outside any loop
}

#[allow(linear_scan_in_loop)]
fn allowed_scan_in_loop() {
    let v: SorobanVec<i32> = SorobanVec(std::vec::Vec::new());
    let target = 5;
    for _ in 0..10 {
        let _ = v.contains(&target); // Good (allowed)
    }
}

fn main() {}
