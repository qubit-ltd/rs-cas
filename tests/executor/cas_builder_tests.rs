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
    HIGH_CONCURRENCY_MAX_ATTEMPTS, HIGH_RELIABILITY_MAX_ATTEMPTS, LOW_LATENCY_MAX_ATTEMPTS,
};
use qubit_cas::{CasExecutor, CasTimeoutPolicy};
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

/// Verifies built-in presets install the expected attempt budgets.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_builder_presets_work() {
    let high_concurrency = CasExecutor::<usize, TestError>::high_concurrency();
    assert_eq!(
        high_concurrency.options().max_attempts(),
        HIGH_CONCURRENCY_MAX_ATTEMPTS
    );

    let low_latency = CasExecutor::<usize, TestError>::low_latency();
    assert_eq!(
        low_latency.options().max_attempts(),
        LOW_LATENCY_MAX_ATTEMPTS
    );

    let high_reliability = CasExecutor::<usize, TestError>::high_reliability();
    assert_eq!(
        high_reliability.options().max_attempts(),
        HIGH_RELIABILITY_MAX_ATTEMPTS
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
