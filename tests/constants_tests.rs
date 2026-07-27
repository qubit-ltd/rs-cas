// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::time::Duration;

use qubit_cas::constants::{
    CONTENTION_ADAPTIVE_INITIAL_DELAY, CONTENTION_ADAPTIVE_JITTER_FACTOR,
    CONTENTION_ADAPTIVE_MAX_ATTEMPTS, CONTENTION_ADAPTIVE_MAX_DELAY,
    CONTENTION_ADAPTIVE_MAX_ELAPSED, CONTENTION_ADAPTIVE_MAX_TOTAL_ELAPSED,
    DEFAULT_CAS_MAX_ATTEMPTS, LATENCY_FIRST_MAX_ATTEMPTS, LATENCY_FIRST_MAX_ELAPSED,
    RELIABILITY_FIRST_INITIAL_DELAY, RELIABILITY_FIRST_JITTER_FACTOR,
    RELIABILITY_FIRST_MAX_ATTEMPTS, RELIABILITY_FIRST_MAX_DELAY, RELIABILITY_FIRST_MAX_ELAPSED,
    RELIABILITY_FIRST_MAX_TOTAL_ELAPSED,
};
use qubit_retry::constants::DEFAULT_RETRY_MAX_ATTEMPTS;

#[test]
fn test_cas_constants_match_retry_default_and_strategy_budgets() {
    assert_eq!(DEFAULT_CAS_MAX_ATTEMPTS, DEFAULT_RETRY_MAX_ATTEMPTS);

    assert_eq!(LATENCY_FIRST_MAX_ATTEMPTS, 100);
    assert_eq!(LATENCY_FIRST_MAX_ELAPSED, Duration::from_secs(5));

    assert_eq!(CONTENTION_ADAPTIVE_MAX_ATTEMPTS, 1000);
    assert_eq!(CONTENTION_ADAPTIVE_INITIAL_DELAY, Duration::from_millis(50));
    assert_eq!(CONTENTION_ADAPTIVE_MAX_DELAY, Duration::from_secs(30));
    assert_eq!(CONTENTION_ADAPTIVE_MAX_ELAPSED, Duration::from_secs(60));
    assert_eq!(
        CONTENTION_ADAPTIVE_MAX_TOTAL_ELAPSED,
        Duration::from_secs(180)
    );
    assert_eq!(CONTENTION_ADAPTIVE_JITTER_FACTOR, 0.25);

    assert_eq!(RELIABILITY_FIRST_MAX_ATTEMPTS, 5000);
    assert_eq!(RELIABILITY_FIRST_INITIAL_DELAY, Duration::from_secs(1));
    assert_eq!(RELIABILITY_FIRST_MAX_DELAY, Duration::from_secs(300));
    assert_eq!(RELIABILITY_FIRST_MAX_ELAPSED, Duration::from_secs(600));
    assert_eq!(
        RELIABILITY_FIRST_MAX_TOTAL_ELAPSED,
        Duration::from_secs(900)
    );
    assert_eq!(RELIABILITY_FIRST_JITTER_FACTOR, 0.1);
}

#[test]
fn test_total_elapsed_budgets_exceed_operation_budgets() {
    assert!(CONTENTION_ADAPTIVE_MAX_TOTAL_ELAPSED > CONTENTION_ADAPTIVE_MAX_ELAPSED);
    assert!(RELIABILITY_FIRST_MAX_TOTAL_ELAPSED > RELIABILITY_FIRST_MAX_ELAPSED);
}
