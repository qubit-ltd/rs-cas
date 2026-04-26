/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use std::time::Duration;

use qubit_cas::constants::{
    CONTENTION_ADAPTIVE_MAX_ATTEMPTS, LATENCY_FIRST_MAX_ATTEMPTS, RELIABILITY_FIRST_MAX_ATTEMPTS,
};
use qubit_cas::{
    CasExecutor, CasObservabilityConfig, CasObservabilityMode, CasStrategy, CasTimeoutPolicy,
    ContentionThresholds, ListenerPanicPolicy,
};
use qubit_retry::{RetryDelay, RetryJitter};

use crate::support::TestError;

/// Verifies builder defaults, helpers, and timeout settings work together.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_builder_default_and_delay_helpers_work() {
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_retries(2)
        .fixed_delay(Duration::from_millis(1))
        .jitter_factor(0.0)
        .attempt_timeout(Some(Duration::from_millis(10)))
        .abort_on_timeout()
        .build()
        .expect("executor should build");

    assert_eq!(executor.options().max_attempts(), 3);
    assert_eq!(
        executor.options().delay(),
        &RetryDelay::fixed(Duration::from_millis(1))
    );
    assert_eq!(executor.options().jitter(), RetryJitter::factor(0.0));
    assert_eq!(executor.attempt_timeout(), Some(Duration::from_millis(10)));
    assert_eq!(executor.timeout_policy(), CasTimeoutPolicy::Abort);
}

/// Verifies built-in strategies install the expected attempt budgets.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_builder_strategies_work() {
    let contention_adaptive = CasExecutor::<usize, TestError>::contention_adaptive();
    assert_eq!(
        contention_adaptive.options().max_attempts(),
        CONTENTION_ADAPTIVE_MAX_ATTEMPTS
    );

    let latency_first = CasExecutor::<usize, TestError>::latency_first();
    assert_eq!(
        latency_first.options().max_attempts(),
        LATENCY_FIRST_MAX_ATTEMPTS
    );

    let reliability_first =
        CasExecutor::<usize, TestError>::with_strategy(CasStrategy::ReliabilityFirst);
    assert_eq!(
        reliability_first.options().max_attempts(),
        RELIABILITY_FIRST_MAX_ATTEMPTS
    );
}

/// Verifies observability settings are installed by the builder.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_builder_observability_settings_work() {
    let thresholds = ContentionThresholds::new(3, 1, 0.5);
    let executor = CasExecutor::<usize, TestError>::builder()
        .observability(
            CasObservabilityConfig::event_stream()
                .with_listener_panic_policy(ListenerPanicPolicy::Isolate),
        )
        .alert_on_contention(thresholds)
        .build()
        .expect("executor should build");

    assert_eq!(
        executor.observability().mode(),
        CasObservabilityMode::EventStreamWithAlert
    );
    assert_eq!(
        executor.observability().listener_panic_policy(),
        ListenerPanicPolicy::Isolate
    );
    assert_eq!(
        executor.observability().contention_thresholds(),
        Some(thresholds)
    );
}

/// Verifies invalid attempt counts are rejected at build time.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_builder_validates_max_attempts() {
    let error = CasExecutor::<usize, TestError>::builder()
        .max_attempts(0)
        .build()
        .expect_err("zero max attempts should be rejected");
    assert!(error.to_string().contains("max_attempts"));
}
