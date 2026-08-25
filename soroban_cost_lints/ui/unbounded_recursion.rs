#![allow(dead_code, unused_variables, unused_mut, unused_parens)]

// Direct recursion over a caller-supplied Vec length: UNBOUNDED -> should warn.
fn walk_vec(data: Vec<u32>) {
    if data.is_empty() {
        return;
    }
    walk_vec(data[1..].to_vec());
}

// Direct recursion over a caller-supplied slice: UNBOUNDED -> should warn.
fn walk_slice(items: &[u32]) {
    if items.is_empty() {
        return;
    }
    walk_slice(&items[1..]);
}

// Mutual recursion through a caller-supplied slice: UNBOUNDED -> should warn.
fn process(items: &[u32]) {
    if items.is_empty() {
        return;
    }
    process_child(items);
}

fn process_child(items: &[u32]) {
    process(&items[1..]);
}

// Direct recursion with a decrementing counter (countdown): BOUNDED -> no warn.
fn countdown(n: u32) {
    if n == 0 {
        return;
    }
    countdown(n - 1);
}

// Direct recursion with a compile-time-constant argument: BOUNDED -> no warn.
fn fixed_depth(n: u32) {
    if n == 0 {
        return;
    }
    fixed_depth(3);
}

// Direct recursion over a fixed-size array literal: BOUNDED -> no warn.
fn fixed_array(a: [u32; 3]) {
    if a[0] == 0 {
        return;
    }
    fixed_array([0, 0, 0]);
}

// Plain integer parameter threaded through: cannot prove bound -> no warn.
fn passthrough(n: u32) {
    if n == 0 {
        return;
    }
    passthrough(n - 1);
}

fn main() {}
