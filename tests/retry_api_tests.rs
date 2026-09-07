// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Consumer-facing tests for the public retry facade.

use std::time::Duration;

use qubit_cas::CasExecutor;
use qubit_cas::retry::BackoffPolicy;
use qubit_cas::retry::RetryCallbackFailure;
use qubit_cas::retry::RetryCancellationPhase;
use qubit_cas::retry::RetryInfrastructureFailure;
use qubit_cas::retry::RetryLimitKind;
use qubit_cas::retry::RetryPolicy;
use qubit_cas::retry::RetryPolicyError;
use qubit_cas::retry::RetryTimeoutScope;

#[test]
fn retry_facade_preserves_policy_type_identity() {
    let backoff = BackoffPolicy::fixed(Duration::from_millis(1));
    let policy = RetryPolicy::builder()
        .max_attempts(2)
        .backoff(backoff)
        .build()
        .expect("valid retry policy");
    let executor = CasExecutor::<usize, &'static str>::from_policy(policy.clone());

    assert_eq!(executor.policy(), &policy);
    assert!(matches!(RetryTimeoutScope::Attempt, RetryTimeoutScope::Attempt));
}

#[test]
fn retry_facade_exports_public_diagnostic_types() {
    let _callback_failure: Option<RetryCallbackFailure> = None;
    let _cancellation_phase: Option<RetryCancellationPhase> = None;
    let _infrastructure_failure: Option<RetryInfrastructureFailure> = None;
    let _limit_kind: Option<RetryLimitKind> = None;
    let _policy_error: Option<RetryPolicyError> = None;
}
