#![warn(vec_where_slice_could_be_used)]

pub mod soroban_sdk {
    pub struct Vec<T>(std::marker::PhantomData<T>);
    impl<T> Vec<T> {
        pub fn new() -> Self { Self(std::marker::PhantomData) }
        pub fn push_back(&mut self, _val: T) {}
    }
}
use soroban_sdk::Vec;

// This should trigger the warning because `v` is never mutated, only moved?
// Wait, the lint says: "If the Vec is mutated anywhere in the function body, it genuinely needs ownership — skip."
// And "Known gap: `mutated_variables` tracks explicit mutations (e.g. `push_back`) but not moves (passing the Vec to another function by value, or returning it). A function that moves the Vec elsewhere genuinely consumes it and should not be flagged, but today it will be. This is acceptable for an initial implementation"
// So passing it to another function WILL trigger it, which is a false positive!
fn bad_read_only(v: Vec<u32>) {
    // read only usage (mocked)
    let _ = &v;
}

fn good_mutated(mut v: Vec<u32>) {
    v.push_back(1); // mutated, should not trigger
}

#[allow(vec_where_slice_could_be_used)]
fn good_moved(v: Vec<u32>) {
    // False positive: v is moved into takes_vec, so ownership is genuinely needed,
    // but the lint currently flags it because it doesn't track moves.
    takes_vec(v);
}

fn takes_vec(_v: Vec<u32>) {}

fn main() {}
