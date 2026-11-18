//! Unit tests for benchmark environment-variable validation.

#[path = "../benches/bench_env.rs"]
#[allow(dead_code)]
mod bench_env;
use bench_env::{parse_iterations, parse_threshold};

#[test]
fn iterations_reject_malformed() {
    assert_eq!(parse_iterations("5"), Ok(5));
    for bad in ["", "abc", "0", "-1", "1.5"] {
        assert!(parse_iterations(bad).is_err(), "{bad}");
    }
}

#[test]
fn threshold_rejects_nan_negative_inf() {
    assert_eq!(parse_threshold("12.5"), Ok(12.5));
    assert_eq!(parse_threshold("0"), Ok(0.0));
    for bad in ["NaN", "nan", "-1", "-0.5", "inf", "abc", ""] {
        assert!(parse_threshold(bad).is_err(), "{bad}");
    }
}
