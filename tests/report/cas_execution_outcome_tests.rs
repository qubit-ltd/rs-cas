/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use qubit_atomic::AtomicRef;
use qubit_cas::{CasDecision, CasErrorKind, CasExecutionOutcome, CasExecutor};

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

    let outcome = executor
        .execute(&state, |current: &usize| CasDecision::update(*current + 1, ()));
    let report = outcome.report().clone();
    let _success = outcome.expect("execution should succeed");

    assert_eq!(report.outcome(), CasExecutionOutcome::SuccessUpdated);
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
    let report = outcome.report().clone();
    let error = outcome.expect_err("retry exhaustion should fail");

    assert_eq!(error.kind(), CasErrorKind::RetryExhausted);
    assert_eq!(report.outcome(), CasExecutionOutcome::ErrorRetryExhausted);
}
