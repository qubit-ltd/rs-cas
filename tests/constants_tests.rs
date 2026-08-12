// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::time::Duration;

use qubit_cas::constants::CONTENTION_ADAPTIVE_INITIAL_DELAY;
use qubit_cas::constants::CONTENTION_ADAPTIVE_JITTER_FACTOR;
use qubit_cas::constants::CONTENTION_ADAPTIVE_MAX_ATTEMPTS;
use qubit_cas::constants::CONTENTION_ADAPTIVE_MAX_DELAY;
use qubit_cas::constants::CONTENTION_ADAPTIVE_MAX_ELAPSED;
use qubit_cas::constants::CONTENTION_ADAPTIVE_MAX_TOTAL_ELAPSED;
use qubit_cas::constants::DEFAULT_CAS_MAX_ATTEMPTS;
use qubit_cas::constants::LATENCY_FIRST_MAX_ATTEMPTS;
use qubit_cas::constants::LATENCY_FIRST_MAX_ELAPSED;
use qubit_cas::constants::LATENCY_FIRST_MAX_TOTAL_ELAPSED;
use qubit_cas::constants::RELIABILITY_FIRST_INITIAL_DELAY;
use qubit_cas::constants::RELIABILITY_FIRST_JITTER_FACTOR;
use qubit_cas::constants::RELIABILITY_FIRST_MAX_ATTEMPTS;
use qubit_cas::constants::RELIABILITY_FIRST_MAX_DELAY;
use qubit_cas::constants::RELIABILITY_FIRST_MAX_ELAPSED;
use qubit_cas::constants::RELIABILITY_FIRST_MAX_TOTAL_ELAPSED;
#[test]
fn test_cas_constants_match_strategy_budgets() {
    assert_eq!(DEFAULT_CAS_MAX_ATTEMPTS, 5);

    assert_eq!(LATENCY_FIRST_MAX_ATTEMPTS, 100);
    assert_eq!(LATENCY_FIRST_MAX_ELAPSED, Duration::from_millis(5));
    assert_eq!(LATENCY_FIRST_MAX_TOTAL_ELAPSED, Duration::from_millis(20));

    assert_eq!(CONTENTION_ADAPTIVE_MAX_ATTEMPTS, 64);
    assert_eq!(CONTENTION_ADAPTIVE_INITIAL_DELAY, Duration::from_micros(50));
    assert_eq!(CONTENTION_ADAPTIVE_MAX_DELAY, Duration::from_millis(5));
    assert_eq!(CONTENTION_ADAPTIVE_MAX_ELAPSED, Duration::from_millis(50));
    assert_eq!(
        CONTENTION_ADAPTIVE_MAX_TOTAL_ELAPSED,
        Duration::from_millis(250)
    );
    assert_eq!(CONTENTION_ADAPTIVE_JITTER_FACTOR, 0.25);

    assert_eq!(RELIABILITY_FIRST_MAX_ATTEMPTS, 128);
    assert_eq!(RELIABILITY_FIRST_INITIAL_DELAY, Duration::from_millis(1));
    assert_eq!(RELIABILITY_FIRST_MAX_DELAY, Duration::from_millis(100));
    assert_eq!(RELIABILITY_FIRST_MAX_ELAPSED, Duration::from_secs(5));
    assert_eq!(RELIABILITY_FIRST_MAX_TOTAL_ELAPSED, Duration::from_secs(10));
    assert_eq!(RELIABILITY_FIRST_JITTER_FACTOR, 0.1);
}

#[test]
fn test_total_elapsed_budgets_exceed_operation_budgets() {
    assert!(LATENCY_FIRST_MAX_TOTAL_ELAPSED > LATENCY_FIRST_MAX_ELAPSED);
    assert!(
        CONTENTION_ADAPTIVE_MAX_TOTAL_ELAPSED > CONTENTION_ADAPTIVE_MAX_ELAPSED
    );
    assert!(
        RELIABILITY_FIRST_MAX_TOTAL_ELAPSED > RELIABILITY_FIRST_MAX_ELAPSED
    );
}
