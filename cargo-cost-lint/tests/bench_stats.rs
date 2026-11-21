//! Unit tests for benchmark statistics helpers.

#[path = "../benches/bench_stats.rs"]
#[allow(dead_code)]
mod bench_stats;

use bench_stats::{median_ms, p95_ms, sample_percentile};
use std::time::Duration;

#[test]
fn median_of_three_returns_middle() {
    let samples = vec![
        Duration::from_millis(10),
        Duration::from_millis(20),
        Duration::from_millis(30),
    ];
    assert_eq!(sample_percentile(&samples, 50), Duration::from_millis(20));
}

#[test]
fn p95_of_twenty_samples() {
    // Sorted 1–20 ms. Index = (19 * 95).div_ceil(100) = 1805/100 = 19 → 20 ms.
    let samples: Vec<Duration> = (1u64..=20).map(Duration::from_millis).collect();
    let p95 = sample_percentile(&samples, 95);
    assert_eq!(p95, Duration::from_millis(20));
}

#[test]
fn single_element_slice_does_not_panic() {
    let samples = vec![Duration::from_millis(42)];
    assert_eq!(sample_percentile(&samples, 95), Duration::from_millis(42));
    assert_eq!(sample_percentile(&samples, 50), Duration::from_millis(42));
}

#[test]
fn p95_exceeds_median_on_skewed_distribution() {
    let samples: Vec<Duration> = (1u64..=100).map(Duration::from_millis).collect();
    let med = median_ms(&samples);
    let p95 = p95_ms(&samples);
    assert!(
        p95 > med,
        "p95 ({p95:.2} ms) should exceed median ({med:.2} ms) for a skewed distribution"
    );
}

#[test]
fn p95_ms_and_median_ms_match_sample_percentile() {
    let samples: Vec<Duration> = (1u64..=10).map(Duration::from_millis).collect();
    let expected_median = sample_percentile(&samples, 50).as_secs_f64() * 1000.0;
    let expected_p95 = sample_percentile(&samples, 95).as_secs_f64() * 1000.0;
    assert_eq!(median_ms(&samples), expected_median);
    assert_eq!(p95_ms(&samples), expected_p95);
}
