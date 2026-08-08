// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::error::Error;
use std::fmt;

use qubit_atomic::AtomicRef;
use qubit_cas::CasDecision;
use qubit_cas::CasErrorKind;
use qubit_cas::CasExecutionOutcome;
use qubit_cas::CasExecutor;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestError(&'static str);

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Error for TestError {}

struct NonDebugError;

#[test]
fn test_cas_outcome_success_accessors_and_parts() {
    let state = AtomicRef::from_value(1usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .no_delay()
        .build()
        .expect("executor should build");

    let outcome = executor.execute(&state, |current: &usize| {
        CasDecision::update(*current + 1, "updated")
    });

    assert!(outcome.is_ok());
    assert!(!outcome.is_err());
    assert!(outcome.result().is_ok());
    assert_eq!(
        outcome.report().outcome(),
        CasExecutionOutcome::SuccessUpdated
    );

    let (result, report) = outcome.clone().into_parts();
    assert!(result.is_ok());
    assert_eq!(report.outcome(), CasExecutionOutcome::SuccessUpdated);

    let success = outcome.expect("execution should succeed");
    assert_eq!(success.into_output(), "updated");
}

#[test]
fn test_cas_outcome_error_accessors_and_result() {
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
    assert_eq!(
        outcome.report().outcome(),
        CasExecutionOutcome::ErrorRetryExhausted
    );

    let result = outcome.clone().into_result();
    assert!(result.is_err());

    let error = outcome.expect_err("retry exhaustion should fail");
    assert_eq!(error.kind(), CasErrorKind::RetryExhausted);
}

/// Verifies `expect` does not require the business error to implement `Debug`.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
#[should_panic(expected = "non-debug error should still support expect")]
fn test_cas_outcome_expect_accepts_non_debug_error() {
    let state = AtomicRef::from_value(1usize);
    let executor = CasExecutor::<usize, NonDebugError>::builder()
        .no_delay()
        .build()
        .expect("executor should build");

    let outcome = executor.execute(&state, |_current: &usize| {
        CasDecision::<usize, (), NonDebugError>::retry(NonDebugError)
    });

    let _ = outcome.expect("non-debug error should still support expect");
}
