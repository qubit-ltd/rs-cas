// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasErrorKind;
use qubit_cas::CasExecutionOutcome;
use qubit_cas::CasExecutor;

use crate::support::TestError;

/// Verifies a successful update execution reports `SuccessUpdated`.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_outcome_reports_success_updated() {
    let state = AtomicRef::from_value(1usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .no_delay()
        .build()
        .expect("executor should build");

    let outcome = executor.execute(&state, |current: &usize| {
        CasDecision::update(*current + 1, ())
    });
    assert!(outcome.is_ok());
    assert!(!outcome.is_err());
    assert!(outcome.result().is_ok());
    let report = outcome.report().clone();
    let (result, report_from_parts) = outcome.clone().into_parts();
    let success = outcome.expect("execution should succeed");

    assert_eq!(report.outcome(), CasExecutionOutcome::SuccessUpdated);
    assert_eq!(
        report_from_parts.outcome(),
        CasExecutionOutcome::SuccessUpdated
    );
    assert!(result.is_ok());
    assert_eq!(success.into_output(), ());
}

/// Verifies retry exhaustion maps to `ErrorRetryExhausted`.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_outcome_reports_retry_exhausted() {
    let state = AtomicRef::from_value(1usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .no_delay()
        .build()
        .expect("executor should build");

    let outcome = executor.execute(&state, |_current: &usize| {
        CasDecision::<usize, (), TestError>::retry(TestError("busy"))
    });
    assert!(!outcome.is_ok());
    assert!(outcome.is_err());
    assert!(outcome.result().is_err());
    let report = outcome.report().clone();
    let result = outcome.clone().into_result();
    let error = outcome.expect_err("retry exhaustion should fail");

    assert!(result.is_err());
    assert_eq!(error.kind(), CasErrorKind::RetryExhausted);
    assert_eq!(report.outcome(), CasExecutionOutcome::ErrorRetryExhausted);
}
