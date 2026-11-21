//! Benchmark statistics helpers (shared with unit tests).

use std::time::Duration;

/// Returns the value at `percentile` (0–100) from a **sorted** slice of
/// `Duration` samples using ceiling-division for the index.
///
/// For example, the 50th percentile of a 3-element slice returns the middle
/// element (index 1) rather than the lower one (index 0).
///
/// # Panics
///
/// Panics when `sorted_samples` is empty.
pub fn sample_percentile(sorted_samples: &[Duration], percentile: usize) -> Duration {
    assert!(!sorted_samples.is_empty(), "sample slice must not be empty");
    let index = ((sorted_samples.len() - 1) * percentile).div_ceil(100);
    sorted_samples[index]
}

/// Returns the median of a sorted sample slice as milliseconds.
pub fn median_ms(sorted_samples: &[Duration]) -> f64 {
    sample_percentile(sorted_samples, 50).as_secs_f64() * 1000.0
}

/// Returns the 95th percentile of a sorted sample slice as milliseconds.
pub fn p95_ms(sorted_samples: &[Duration]) -> f64 {
    sample_percentile(sorted_samples, 95).as_secs_f64() * 1000.0
}
