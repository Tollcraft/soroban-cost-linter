//! Validation of benchmark environment knobs (shared with unit tests).

/// Parses the raw `LINTER_BENCH_ITERATIONS` value; must be a positive integer.
pub fn parse_iterations(raw: &str) -> Result<usize, String> {
    match raw.trim().parse::<usize>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(format!(
            "LINTER_BENCH_ITERATIONS must be a positive integer, got {raw:?}"
        )),
    }
}

/// Parses the raw `BENCHMARK_THRESHOLD` value; must be a finite, non-negative number.
pub fn parse_threshold(raw: &str) -> Result<f64, String> {
    match raw.trim().parse::<f64>() {
        Ok(v) if v.is_finite() && v >= 0.0 => Ok(v),
        _ => Err(format!(
            "BENCHMARK_THRESHOLD must be a finite, non-negative number, got {raw:?}"
        )),
    }
}
