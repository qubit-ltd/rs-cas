// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::time::Duration;

use qubit_cas::CasExecutor;
use qubit_cas::CasStrategy;
use qubit_cas::ContentionThresholds;
use qubit_cas::ListenerPanicPolicy;
use qubit_cas::executor::CasBuilder;
use qubit_cas::observability::CasObservabilityConfig;
use qubit_retry::BackoffPolicy;
use qubit_retry::RetryPolicy;

use crate::support::TestError;

#[test]
fn test_build_exposes_pure_policy_and_attempt_timeout() {
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .fixed_delay(Duration::from_millis(2))
        .attempt_timeout(Some(Duration::from_millis(10)))
        .build()
        .expect("CAS policy should be valid");

    assert_eq!(executor.policy().limits().max_attempts().get(), 3);
    assert_eq!(
        executor.policy().backoff().maximum_delay(),
        Some(Duration::from_millis(2))
    );
    assert_eq!(executor.attempt_timeout(), Some(Duration::from_millis(10)));
}

#[test]
fn test_from_policy_preserves_validated_policy() {
    let policy = RetryPolicy::builder()
        .max_attempts(7)
        .backoff(BackoffPolicy::fixed(Duration::from_millis(4)))
        .build()
        .expect("retry policy should be valid");
    let executor = CasExecutor::<usize, TestError>::from_policy(policy.clone());

    assert_eq!(executor.policy(), &policy);
}

#[test]
fn test_build_reports_invalid_backoff() {
    let error = CasExecutor::<usize, TestError>::builder()
        .random_delay(Duration::from_millis(2), Duration::from_millis(1))
        .build()
        .expect_err("reversed random bounds should fail");

    assert_eq!(error.field(), "backoff.uniform");
}

/// Verifies the builder exposes all retry and observability configuration
/// paths.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_builder_configuration_methods_compose() {
    let thresholds = ContentionThresholds::new(2, 1, 0.5);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(4)
        .max_retries(3)
        .max_operation_elapsed(Some(Duration::from_secs(2)))
        .max_total_elapsed(Some(Duration::from_secs(3)))
        .backoff(BackoffPolicy::immediate())
        .no_delay()
        .fixed_delay(Duration::from_millis(1))
        .random_delay(Duration::from_millis(1), Duration::from_millis(2))
        .exponential_backoff(Duration::from_millis(1), Duration::from_millis(4))
        .exponential_backoff_with_multiplier(Duration::from_millis(1), Duration::from_millis(4), 2.0)
        .jitter_factor(0.1)
        .attempt_timeout(None)
        .retry_on_timeout()
        .abort_on_timeout()
        .observability(CasObservabilityConfig::event_stream())
        .alert_on_contention(thresholds)
        .isolate_listener_panics()
        .build()
        .expect("composed builder should build");

    assert_eq!(executor.policy().limits().max_attempts().get(), 4);
    assert_eq!(
        executor.observability().listener_panic_policy(),
        ListenerPanicPolicy::Isolate
    );
    assert_eq!(executor.observability().contention_thresholds(), Some(thresholds));
    assert_eq!(CasStrategy::default(), CasStrategy::LatencyFirst);
    let _default_builder = CasBuilder::<usize, TestError>::default();
}

/// Verifies the built-in strategy builder helpers produce valid executors.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_builtin_strategy_builders_succeed() {
    CasExecutor::<usize, TestError>::builder()
        .build_contention_adaptive()
        .expect("contention-adaptive builder should succeed");
    CasExecutor::<usize, TestError>::builder()
        .build_latency_first()
        .expect("latency-first builder should succeed");
    CasExecutor::<usize, TestError>::builder()
        .build_reliability_first()
        .expect("reliability-first builder should succeed");
    CasExecutor::<usize, TestError>::builder()
        .strategy(CasStrategy::ReliabilityFirst)
        .build()
        .expect("strategy builder should succeed");
}
