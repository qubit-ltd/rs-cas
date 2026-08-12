// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::time::Duration;

use qubit_cas::CasExecutor;
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
