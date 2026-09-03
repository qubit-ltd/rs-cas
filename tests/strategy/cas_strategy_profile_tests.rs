// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_cas::CasStrategy;
use qubit_cas::CasStrategyProfile;
use qubit_cas::constants::CONTENTION_ADAPTIVE_MAX_ELAPSED;
use qubit_cas::constants::CONTENTION_ADAPTIVE_MAX_TOTAL_ELAPSED;
use qubit_cas::constants::LATENCY_FIRST_MAX_ATTEMPTS;
use qubit_cas::constants::LATENCY_FIRST_MAX_ELAPSED;
use qubit_cas::constants::LATENCY_FIRST_MAX_TOTAL_ELAPSED;
use qubit_cas::constants::RELIABILITY_FIRST_MAX_ATTEMPTS;
use qubit_cas::constants::RELIABILITY_FIRST_MAX_ELAPSED;
use qubit_cas::constants::RELIABILITY_FIRST_MAX_TOTAL_ELAPSED;

#[test]
fn test_cas_strategy_profile_accessors() {
    let latency = CasStrategy::LatencyFirst.profile();
    assert_eq!(latency.max_attempts(), LATENCY_FIRST_MAX_ATTEMPTS);
    assert_eq!(latency.max_operation_elapsed(), LATENCY_FIRST_MAX_ELAPSED);
    assert_eq!(latency.max_total_elapsed(), Some(LATENCY_FIRST_MAX_TOTAL_ELAPSED));
    assert!(!latency.uses_backoff());

    let contention = CasStrategy::ContentionAdaptive.profile();
    assert!(contention.max_attempts() > 0);
    assert_eq!(contention.max_operation_elapsed(), CONTENTION_ADAPTIVE_MAX_ELAPSED);
    assert_eq!(
        contention.max_total_elapsed(),
        Some(CONTENTION_ADAPTIVE_MAX_TOTAL_ELAPSED)
    );
    assert!(contention.uses_backoff());

    let reliability = CasStrategy::ReliabilityFirst.profile();
    assert!(reliability.max_attempts() > 0);
    assert_eq!(reliability.max_operation_elapsed(), RELIABILITY_FIRST_MAX_ELAPSED);
    assert_eq!(
        reliability.max_total_elapsed(),
        Some(RELIABILITY_FIRST_MAX_TOTAL_ELAPSED)
    );
    assert!(reliability.uses_backoff());
}

/// Verifies strategy profile accessors can be used through function pointers.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_strategy_profile_accessor_function_pointers_work() {
    let profile = CasStrategy::ReliabilityFirst.profile();
    let max_attempts: fn(&CasStrategyProfile) -> u32 = CasStrategyProfile::max_attempts;
    let max_operation_elapsed: fn(&CasStrategyProfile) -> std::time::Duration =
        CasStrategyProfile::max_operation_elapsed;
    let uses_backoff: fn(&CasStrategyProfile) -> bool = CasStrategyProfile::uses_backoff;

    assert_eq!(max_attempts(&profile), RELIABILITY_FIRST_MAX_ATTEMPTS);
    assert_eq!(max_operation_elapsed(&profile), RELIABILITY_FIRST_MAX_ELAPSED);
    assert!(uses_backoff(&profile));
}
