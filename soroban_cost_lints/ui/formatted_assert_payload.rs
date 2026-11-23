#![warn(formatted_assert_payload)]
#![allow(clippy::all)]
#![allow(unused)]

fn bad_assert(ok: bool, value: u32) {
    assert!(ok, "bad {}", value); //~ WARNING assert macro formats its failure message eagerly
}

fn bad_debug_assert(ok: bool, value: u32) {
    debug_assert!(ok, "bad {}", value); //~ WARNING assert macro formats its failure message eagerly
}

fn bad_assert_eq(left: u32, right: u32, value: u32) {
    assert_eq!(left, right, "bad {}", value); //~ WARNING assert macro formats its failure message eagerly
}

fn bad_assert_ne(left: u32, right: u32, value: u32) {
    assert_ne!(left, right, "bad {}", value); //~ WARNING assert macro formats its failure message eagerly
}

fn good_plain_assert(ok: bool) {
    assert!(ok);
}

fn good_plain_literal(ok: bool) {
    assert!(ok, "plain literal");
}

fn good_plain_eq(left: u32, right: u32) {
    assert_eq!(left, right);
}

#[allow(formatted_assert_payload)]
fn allowed(ok: bool, value: u32) {
    assert!(ok, "bad {}", value);
}

#[cfg(test)]
mod tests {
    fn in_test_module(ok: bool, value: u32) {
        assert!(ok, "bad {}", value);
    }

    #[test]
    fn test_case(left: u32, right: u32, value: u32) {
        assert_eq!(left, right, "bad {}", value);
    }
}

fn main() {}
