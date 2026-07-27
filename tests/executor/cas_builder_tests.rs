// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::time::Duration;

use qubit_cas::constants::{
    CONTENTION_ADAPTIVE_MAX_ATTEMPTS, DEFAULT_CAS_MAX_ATTEMPTS, LATENCY_FIRST_MAX_ATTEMPTS,
    RELIABILITY_FIRST_MAX_ATTEMPTS,
};
use qubit_cas::{
    CasBuilder, CasExecutor, CasObservabilityConfig, CasObservabilityMode, CasStrategy,
    ContentionThresholds, ListenerPanicPolicy,
};
use qubit_retry::{AttemptTimeoutOption, RetryDelay, RetryJitter, RetryOptions};

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
    assert_eq!(
        executor.options().attempt_timeout(),
        Some(AttemptTimeoutOption::abort(Duration::from_millis(10)))
    );
}

/// Verifies builder can adopt retry options and random delay settings.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_builder_options_and_random_delay_work() {
    let options = RetryOptions::new(
        4,
        Some(Duration::from_millis(200)),
        None,
        RetryDelay::fixed(Duration::from_millis(3)),
        RetryJitter::factor(0.25),
    )
    .expect("retry options should be valid");
    let executor = CasExecutor::<usize, TestError>::from_options(options.clone())
        .expect("executor should build from options");

    assert_eq!(executor.options(), &options);

    let random = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .random_delay(Duration::from_millis(1), Duration::from_millis(5))
        .jitter_factor(0.0)
        .build()
        .expect("executor should build with random delay");
    assert_eq!(
        random.options().delay(),
        &RetryDelay::random(Duration::from_millis(1), Duration::from_millis(5))
    );
}

/// Verifies retry option snapshots keep per-attempt timeout settings intact.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_builder_options_preserves_attempt_timeout_option() {
    let options = RetryOptions::new_with_attempt_timeout(
        4,
        Some(Duration::from_millis(200)),
        None,
        RetryDelay::none(),
        RetryJitter::none(),
        Some(AttemptTimeoutOption::abort(Duration::from_millis(7))),
    )
    .expect("retry options should be valid");

    let executor = CasExecutor::<usize, TestError>::from_options(options.clone())
        .expect("executor should build from options");

    assert_eq!(executor.options(), &options);
    assert_eq!(
        executor.options().attempt_timeout(),
        Some(AttemptTimeoutOption::abort(Duration::from_millis(7)))
    );
}

/// Verifies the builder installs complete retry-layer timeout options.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_builder_attempt_timeout_option_work() {
    let executor = CasExecutor::<usize, TestError>::builder()
        .attempt_timeout_option(Some(AttemptTimeoutOption::retry(Duration::from_millis(9))))
        .build()
        .expect("executor should build");

    assert_eq!(
        executor.options().attempt_timeout(),
        Some(AttemptTimeoutOption::retry(Duration::from_millis(9)))
    );
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

/// Verifies listener panic isolation is installed by builder helpers.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_builder_isolate_listener_panics_work() {
    let executor = CasExecutor::<usize, TestError>::builder()
        .isolate_listener_panics()
        .build()
        .expect("executor should build");

    assert_eq!(
        executor.observability().listener_panic_policy(),
        ListenerPanicPolicy::Isolate
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

/// Verifies zero attempt timeouts are rejected at build time.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_builder_validates_attempt_timeout() {
    let error = CasExecutor::<usize, TestError>::builder()
        .attempt_timeout(Some(Duration::ZERO))
        .build()
        .expect_err("zero attempt timeout should be rejected");
    assert!(error.to_string().contains("attempt_timeout"));
}

/// Verifies invalid retry options from delay validation are rejected.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_builder_validates_retry_options() {
    let error = CasExecutor::<usize, TestError>::builder()
        .fixed_delay(Duration::ZERO)
        .build()
        .expect_err("zero fixed delay should be rejected");
    assert!(error.to_string().contains("fixed delay"));
}

/// Verifies default and strategy-specific builder constructors.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_builder_default_and_reliability_first_work() {
    let default_executor = CasBuilder::<usize, TestError>::default()
        .build()
        .expect("default builder should build");
    assert_eq!(
        default_executor.options().max_attempts(),
        DEFAULT_CAS_MAX_ATTEMPTS
    );

    let reliability_executor = CasExecutor::<usize, TestError>::builder()
        .build_reliability_first()
        .expect("reliability-first builder should build");
    assert_eq!(
        reliability_executor.options().max_attempts(),
        RELIABILITY_FIRST_MAX_ATTEMPTS
    );

    let convenience = CasExecutor::<usize, TestError>::reliability_first();
    assert_eq!(
        convenience.options().max_attempts(),
        RELIABILITY_FIRST_MAX_ATTEMPTS
    );
}
