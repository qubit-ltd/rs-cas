// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::CasAttemptFailure;
use qubit_cas::CasDecision;
use qubit_cas::CasErrorKind;
use qubit_cas::CasExecutionOutcome;
use qubit_cas::CasExecutor;
use qubit_retry::RetryErrorReason;

use crate::support::TestError;

/// Verifies terminal CAS errors expose display text and source errors.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_cas_error_display_and_source_work() {
    let state = AtomicRef::from_value(3usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .no_delay()
        .build()
        .expect("executor should build");

    let error = executor
        .execute(&state, |_current: &usize| {
            CasDecision::<usize, (), TestError>::retry(TestError("still-busy"))
        })
        .expect_err("retry exhaustion should fail");

    assert_eq!(error.kind(), CasErrorKind::RetryExhausted);
    assert!(error.to_string().contains("retryable failures exhausted"));
    assert_eq!(error.reason(), RetryErrorReason::AttemptsExhausted);
    assert!(format!("{error:?}").contains("CasError"));
    assert_eq!(
        error.source().map(ToString::to_string),
        Some("still-busy".to_string())
    );
    assert_eq!(error.error(), Some(&TestError("still-busy")));
    assert_eq!(error.current().map(|current| **current), Some(3));
}

/// Verifies terminal errors can transfer their captured failure without
/// cloning it.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_cas_error_into_parts_preserves_owned_failure() {
    let state = AtomicRef::from_value(3usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .no_delay()
        .build()
        .expect("executor should build");

    let error = executor
        .execute_result(&state, |_current: &usize| {
            CasDecision::<usize, (), TestError>::abort(TestError("blocked"))
        })
        .expect_err("abort should fail");

    let (kind, reason, context, last_failure) = error.into_parts();
    assert_eq!(kind, CasErrorKind::Abort);
    assert_eq!(reason, RetryErrorReason::Aborted);
    assert_eq!(context.attempt(), 1);
    match last_failure {
        Some(CasAttemptFailure::Abort { current, error }) => {
            assert_eq!(*current, 3);
            assert_eq!(error, TestError("blocked"));
        }
        other => panic!("expected an owned abort failure, got {other:?}"),
    }
}

/// Verifies terminal errors can consume just their captured failure.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_cas_error_into_last_failure_returns_owned_failure() {
    let state = AtomicRef::from_value(4usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .no_delay()
        .build()
        .expect("executor should build");

    let failure = executor
        .execute_result(&state, |_current: &usize| {
            CasDecision::<usize, (), TestError>::abort(TestError("blocked"))
        })
        .expect_err("abort should fail")
        .into_last_failure();

    assert!(matches!(failure, Some(CasAttemptFailure::Abort { .. })));
}

/// Verifies terminal CAS error display text for non-retry terminal kinds.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_cas_error_display_covers_abort_conflict_and_elapsed_kinds() {
    let abort_state = AtomicRef::from_value(4usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .no_delay()
        .build()
        .expect("executor should build");
    let abort = executor
        .execute(&abort_state, |_current: &usize| {
            CasDecision::<usize, (), TestError>::abort(TestError("blocked"))
        })
        .expect_err("abort should fail");
    assert_eq!(abort.kind(), CasErrorKind::Abort);
    assert_eq!(abort.reason(), RetryErrorReason::Aborted);
    assert!(abort.to_string().contains("CAS aborted"));
    assert_eq!(
        abort.source().map(ToString::to_string),
        Some("blocked".to_string())
    );

    let conflict_state = AtomicRef::from_value(10usize);
    let conflicts = AtomicUsize::new(0);
    let conflict_executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .no_delay()
        .build()
        .expect("executor should build");
    let conflict = conflict_executor
        .execute(&conflict_state, |current: &usize| {
            conflicts.fetch_add(1, Ordering::SeqCst);
            conflict_state.store(Arc::new(*current + 1));
            CasDecision::<usize, (), TestError>::update(*current + 2, ())
        })
        .expect_err("conflicts should exhaust attempts");
    assert_eq!(conflict.kind(), CasErrorKind::Conflict);
    assert_eq!(conflict.reason(), RetryErrorReason::AttemptsExhausted);
    assert!(conflict.to_string().contains("conflicts exhausted"));
    assert_eq!(conflicts.load(Ordering::SeqCst), 2);

    let elapsed_state = AtomicRef::from_value(12usize);
    let elapsed_executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .no_delay()
        .max_operation_elapsed(Some(Duration::from_millis(1)))
        .build()
        .expect("executor should build");
    let op_outcome =
        elapsed_executor.execute(&elapsed_state, |_current: &usize| {
            std::thread::sleep(Duration::from_millis(2));
            CasDecision::<usize, (), TestError>::retry(TestError("slow"))
        });
    assert_eq!(
        op_outcome.report().outcome(),
        CasExecutionOutcome::ErrorMaxOperationElapsedExceeded
    );
    let elapsed = op_outcome
        .into_result()
        .expect_err("operation elapsed budget should fail");
    assert_eq!(elapsed.kind(), CasErrorKind::MaxOperationElapsedExceeded);
    assert_eq!(elapsed.reason(), RetryErrorReason::OperationBudgetExhausted);
    assert!(
        elapsed
            .to_string()
            .contains("max operation elapsed exceeded")
    );

    let total_state = AtomicRef::from_value(13usize);
    let total_executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(5)
        .no_delay()
        .max_operation_elapsed(None)
        .max_total_elapsed(Some(Duration::ZERO))
        .build()
        .expect("executor should build");
    let total_outcome =
        total_executor.execute(&total_state, |_current: &usize| {
            CasDecision::<usize, (), TestError>::retry(TestError("x"))
        });
    assert_eq!(
        total_outcome.report().outcome(),
        CasExecutionOutcome::ErrorMaxTotalElapsedExceeded
    );
    let total = total_outcome
        .into_result()
        .expect_err("total elapsed budget should fail");
    assert_eq!(total.kind(), CasErrorKind::MaxTotalElapsedExceeded);
    assert_eq!(total.reason(), RetryErrorReason::TotalBudgetExhausted);
    assert!(total.to_string().contains("max total elapsed exceeded"));
}

/// Verifies synchronous execution does not expose async timeout controls.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_execute_sync_ignores_async_timeout_configuration() {
    let state = AtomicRef::from_value(21usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .attempt_timeout(Some(Duration::from_millis(10)))
        .abort_on_timeout()
        .build()
        .expect("executor should build");

    let outcome = executor.execute(&state, |_current: &usize| {
        CasDecision::<usize, (), TestError>::finish(())
    });

    assert!(outcome.into_result().is_ok());
}

/// Verifies async attempt timeouts use the timeout terminal error formatting.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[cfg(feature = "tokio")]
#[tokio::test(start_paused = true)]
async fn test_cas_error_display_covers_attempt_timeout_kind() {
    let state = AtomicRef::from_value(8usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .attempt_timeout(Some(Duration::from_millis(10)))
        .abort_on_timeout()
        .build()
        .expect("executor should build");

    let error = executor
        .execute_async(&state, |_current: Arc<usize>| async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            CasDecision::<usize, (), TestError>::finish(())
        })
        .await
        .expect_err("attempt timeout should abort");

    assert_eq!(error.kind(), CasErrorKind::AttemptTimeout);
    assert_eq!(error.reason(), RetryErrorReason::AttemptTimedOut);
    assert!(error.to_string().contains("attempt timed out"));

    let retrying_executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .attempt_timeout(Some(Duration::from_millis(10)))
        .retry_on_timeout()
        .build()
        .expect("executor should build");
    let exhausted = retrying_executor
        .execute_async(&state, |_current: Arc<usize>| async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            CasDecision::<usize, (), TestError>::finish(())
        })
        .await
        .expect_err("repeated attempt timeouts should exhaust attempts");

    assert_eq!(exhausted.kind(), CasErrorKind::AttemptTimeout);
    assert_eq!(exhausted.reason(), RetryErrorReason::AttemptsExhausted);
    assert!(exhausted.to_string().contains("attempt timed out"));
}
