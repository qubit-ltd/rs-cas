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

#[derive(Debug)]
struct NonCloneSnapshot;

#[derive(Debug)]
struct NonCloneOutput {
    value: usize,
}

#[test]
fn test_execution_accepts_non_clone_business_output_and_error() {
    let state = AtomicRef::from_value(NonCloneSnapshot);
    let executor = CasExecutor::<NonCloneSnapshot, NonDebugError>::builder()
        .build()
        .expect("valid builder");
    let success = executor.execute(&state, |_: &NonCloneSnapshot| {
        CasDecision::finish(NonCloneOutput { value: 7 })
    });
    assert_eq!(success.expect("non-clone output succeeds").into_output().value, 7);
    let failure = executor.execute(&state, |_: &NonCloneSnapshot| {
        CasDecision::<_, NonCloneOutput, _>::abort(NonDebugError)
    });
    assert!(failure.expect_err("non-clone error is retained").error().is_some());
}

#[test]
fn test_outcome_clone_accepts_non_clone_snapshot_for_success_and_error() {
    let state = AtomicRef::from_value(NonCloneSnapshot);
    let executor = CasExecutor::<NonCloneSnapshot, String>::builder()
        .build()
        .expect("valid builder");
    let success = executor.execute(&state, |_: &NonCloneSnapshot| CasDecision::finish("done".to_owned()));
    let error = executor.execute(&state, |_: &NonCloneSnapshot| {
        CasDecision::<_, String, _>::abort("stopped".to_owned())
    });
    for outcome in [success, error] {
        let cloned = outcome.clone();
        assert_eq!(cloned.is_ok(), outcome.is_ok());
        assert_eq!(cloned.report().outcome(), outcome.report().outcome());
        assert_eq!(cloned.report().attempts_total(), outcome.report().attempts_total());
        match (cloned.into_result(), outcome.into_result()) {
            (Ok(cloned), Ok(original)) => {
                assert!(std::sync::Arc::ptr_eq(cloned.current(), original.current()));
                assert_eq!(cloned.output(), original.output());
            }
            (Err(cloned), Err(original)) => assert_eq!(cloned.error(), original.error()),
            _ => panic!("clone must preserve terminal outcome"),
        }
    }
}

#[test]
fn test_cas_outcome_success_accessors_and_parts() {
    let state = AtomicRef::from_value(1usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .no_delay()
        .build()
        .expect("executor should build");

    let outcome = executor.execute(&state, |current: &usize| CasDecision::update(*current + 1, "updated"));

    assert!(outcome.is_ok());
    assert!(!outcome.is_err());
    assert!(outcome.result().is_ok());
    assert_eq!(outcome.report().outcome(), CasExecutionOutcome::SuccessUpdated);

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
    assert_eq!(outcome.report().outcome(), CasExecutionOutcome::ErrorRetryExhausted);

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
