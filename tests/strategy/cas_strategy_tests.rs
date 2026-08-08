// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_cas::CasStrategy;
use qubit_cas::constants::CONTENTION_ADAPTIVE_MAX_ATTEMPTS;
use qubit_cas::constants::CONTENTION_ADAPTIVE_MAX_TOTAL_ELAPSED;
use qubit_cas::constants::LATENCY_FIRST_MAX_ATTEMPTS;
use qubit_cas::constants::LATENCY_FIRST_MAX_TOTAL_ELAPSED;
use qubit_cas::constants::RELIABILITY_FIRST_MAX_ATTEMPTS;
use qubit_cas::constants::RELIABILITY_FIRST_MAX_TOTAL_ELAPSED;

/// Verifies strategy defaults and profile accessors.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_strategy_profiles_expose_expected_values() {
    assert_eq!(CasStrategy::default(), CasStrategy::LatencyFirst);

    let latency = CasStrategy::LatencyFirst.profile();
    assert_eq!(latency.max_attempts(), LATENCY_FIRST_MAX_ATTEMPTS);
    assert_eq!(
        latency.max_total_elapsed(),
        Some(LATENCY_FIRST_MAX_TOTAL_ELAPSED)
    );
    assert!(!latency.uses_backoff());

    let contention = CasStrategy::ContentionAdaptive.profile();
    assert_eq!(contention.max_attempts(), CONTENTION_ADAPTIVE_MAX_ATTEMPTS);
    assert_eq!(
        contention.max_total_elapsed(),
        Some(CONTENTION_ADAPTIVE_MAX_TOTAL_ELAPSED)
    );
    assert!(contention.uses_backoff());

    let reliability = CasStrategy::ReliabilityFirst.profile();
    assert_eq!(reliability.max_attempts(), RELIABILITY_FIRST_MAX_ATTEMPTS);
    assert_eq!(
        reliability.max_total_elapsed(),
        Some(RELIABILITY_FIRST_MAX_TOTAL_ELAPSED)
    );
    assert!(reliability.uses_backoff());
}
