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
#[test]
fn test_outcome_reports_success_updated() {
    let state = AtomicRef::from_value(1usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .no_delay()
        .build()
        .expect("executor should build");

    let outcome = executor.execute(&state, |current: &usize| CasDecision::update(*current + 1, ()));
    assert!(outcome.is_ok(), "successful update should report success");
    assert!(!outcome.is_err(), "successful update should not report an error");
    assert!(outcome.result().is_ok(), "borrowed update result should be successful");
    let report = outcome.report().clone();
    let (result, report_from_parts) = outcome.clone().into_parts();
    let success = outcome.expect("execution should succeed");

    assert_eq!(report.outcome(), CasExecutionOutcome::SuccessUpdated);
    assert_eq!(report_from_parts.outcome(), CasExecutionOutcome::SuccessUpdated);
    assert!(result.is_ok(), "decomposed update result should be successful");
    assert_eq!(success.into_output(), ());
}

/// Verifies retry exhaustion maps to `ErrorRetryExhausted`.
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
    assert!(!outcome.is_ok(), "retry exhaustion should not report success");
    assert!(outcome.is_err(), "retry exhaustion should report an error");
    assert!(
        outcome.result().is_err(),
        "borrowed exhausted result should be an error"
    );
    let report = outcome.report().clone();
    let result = outcome.clone().into_result();
    let error = outcome.expect_err("retry exhaustion should fail");

    assert!(result.is_err(), "consumed exhausted result should be an error");
    assert_eq!(error.kind(), CasErrorKind::RetryExhausted);
    assert_eq!(report.outcome(), CasExecutionOutcome::ErrorRetryExhausted);
}
