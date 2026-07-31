// UI test suite for the `formatted_panic_payload` lint rule.
//
// `format!`, formatted `panic!`, and `.expect(&format!(..))` are plain
// core/std, not soroban_sdk-specific, so this fixture needs no SDK stub.

// ===========================================================================
//  Triggering cases — these MUST produce the formatted_panic_payload warning
// ===========================================================================

/// A bare `format!(...)` call always pulls in `core::fmt`.
fn bad_format(a: u32, b: u32) -> String {
    format!("balance {a} below {b}") //~ WARNING formatted panic payload pulls in string-formatting machinery
}

/// `format!(...)` with positional arguments (not just captured identifiers).
fn bad_format_positional(a: u32, b: u32) -> String {
    format!("balance {} below {}", a, b) //~ WARNING formatted panic payload pulls in string-formatting machinery
}

/// `panic!` with formatting arguments builds an `Arguments` value at the
/// panic site.
fn bad_panic_with_args(a: u32, b: u32) {
    panic!("balance {} below {}", a, b); //~ WARNING formatted panic payload pulls in string-formatting machinery
}

/// `.expect(&format!(...))` builds the message eagerly through `format!`.
fn bad_expect_format(value: Option<u32>, key: u32) -> u32 {
    value.expect(&format!("missing {key}")) //~ WARNING formatted panic payload pulls in string-formatting machinery
}

// ===========================================================================
//  Non-triggering cases — these MUST NOT produce a warning
// ===========================================================================

/// `panic!` with a plain literal and zero arguments never touches
/// `core::fmt`.
fn good_panic_plain_literal() {
    panic!("balance too low");
}

/// `.expect("plain literal")` never touches `core::fmt`.
fn good_expect_plain_literal(value: Option<u32>) -> u32 {
    value.expect("missing value")
}

/// `.expect(msg)` where `msg` is just a `&str` local, not a `format!` call.
fn good_expect_local_str(value: Option<u32>, msg: &str) -> u32 {
    value.expect(msg)
}

/// Explicitly suppressed via the standard `#[allow(..)]` attribute.
#[allow(formatted_panic_payload)]
fn allowed_format(a: u32) -> String {
    format!("value {a}")
}

// ===========================================================================
//  Test code — none of the flagged patterns above should fire under
//  `#[cfg(test)]`, whether on the function directly or on an enclosing
//  `mod tests { .. }`.
// ===========================================================================

#[cfg(test)]
fn cfg_test_fn_format(a: u32) -> String {
    format!("value {a}")
}

#[cfg(test)]
fn cfg_test_fn_panic(a: u32, b: u32) {
    panic!("balance {} below {}", a, b);
}

#[cfg(test)]
fn cfg_test_fn_expect(value: Option<u32>, key: u32) -> u32 {
    value.expect(&format!("missing {key}"))
}

#[cfg(test)]
mod tests {
    fn in_test_module_format(a: u32) -> String {
        format!("value {a}")
    }

    fn in_test_module_panic(a: u32, b: u32) {
        panic!("balance {} below {}", a, b);
    }

    fn in_test_module_expect(value: Option<u32>, key: u32) -> u32 {
        value.expect(&format!("missing {key}"))
    }
}

fn main() {}
