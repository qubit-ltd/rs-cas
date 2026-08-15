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
use qubit_cas::CasError;
use qubit_cas::CasErrorKind;
use qubit_cas::CasExecutionOutcome;
use qubit_cas::CasExecutor;
use qubit_cas::CasRetryFailure;
use qubit_clock::Timer;
use qubit_clock::test_util::FaultInjectingTimer;
use qubit_clock::test_util::TimerFailurePoint;
use qubit_retry::AttemptFailure;
use qubit_retry::BackoffPolicy;
use qubit_retry::Retry;
use qubit_retry::RetryCallbackKind;
use qubit_retry::RetryCallbackPhase;
#[cfg(feature = "tokio")]
use qubit_retry::RetryCancellationPhase;
#[cfg(feature = "tokio")]
use qubit_retry::RetryCancellationToken;
use qubit_retry::RetryContext;
use qubit_retry::RetryDecision;
use qubit_retry::RetryInfrastructureFailure;
use qubit_retry::RetryLimitKind;
use qubit_retry::RetryObserver;
use qubit_retry::RetryPolicy;
#[cfg(feature = "tokio")]
use qubit_retry::RetryTimeoutScope;

use crate::support::TestError;

/// Builds a no-delay retry policy for terminal mapping tests.
fn terminal_test_policy(max_attempts: u32) -> RetryPolicy {
    RetryPolicy::builder()
        .max_attempts(max_attempts)
        .backoff(BackoffPolicy::immediate())
        .build()
        .expect("terminal mapping policy should be valid")
}

/// Observer that panics before the first operation is admitted.
struct PanickingStartedObserver;

impl RetryObserver<CasAttemptFailure<usize, TestError>>
    for PanickingStartedObserver
{
    fn on_attempt_started(&self, _context: &RetryContext) {
        panic!("CAS retry observer failed");
    }
}

/// Verifies an abort terminal keeps its business attempt failure.
#[test]
fn test_cas_error_maps_aborted_terminal() {
    let retry_error = Retry::builder(terminal_test_policy(2))
        .rule(
            |_: &AttemptFailure<CasAttemptFailure<usize, TestError>>,
             _: &RetryContext| { RetryDecision::Abort },
        )
        .build()
        .sync()
        .run(|| {
            Err::<(), _>(CasAttemptFailure::Abort {
                current: Arc::new(1),
                error: TestError("blocked"),
            })
        })
        .expect_err("the rule should abort");

    let error = CasError::from(retry_error);
    assert!(matches!(error.failure(), CasRetryFailure::Aborted));
    assert_eq!(error.failure().limit(), None);
    assert_eq!(error.kind(), CasErrorKind::Abort);
    assert_eq!(error.error(), Some(&TestError("blocked")));
}

/// Verifies attempts exhaustion keeps its exact limit and business failure.
#[test]
fn test_cas_error_maps_exhausted_terminal() {
    let retry_error = Retry::builder(terminal_test_policy(1))
        .build()
        .sync()
        .run(|| {
            Err::<(), _>(CasAttemptFailure::Retry {
                current: Arc::new(2),
                error: TestError("busy"),
            })
        })
        .expect_err("one failed attempt should exhaust the policy");

    let error = CasError::from(retry_error);
    assert!(matches!(
        error.failure(),
        CasRetryFailure::Exhausted {
            limit: RetryLimitKind::Attempts,
        }
    ));
    assert_eq!(error.failure().limit(), Some(RetryLimitKind::Attempts));
    assert_eq!(error.kind(), CasErrorKind::RetryExhausted);
    assert_eq!(error.error(), Some(&TestError("busy")));
}

/// Verifies a hard attempt timeout remains structural and has no business E.
#[cfg(feature = "tokio")]
#[tokio::test(start_paused = true)]
async fn test_cas_error_maps_timed_out_terminal_without_business_error() {
    let retry_error = Retry::<CasAttemptFailure<usize, TestError>>::builder(
        terminal_test_policy(1),
    )
    .build()
    .asynchronous()
    .attempt_timeout(Duration::from_millis(1))
    .run(
        std::future::pending::<Result<(), CasAttemptFailure<usize, TestError>>>,
    )
    .await
    .expect_err("the pending attempt should time out");

    let error = CasError::from(retry_error);
    assert!(matches!(
        error.failure(),
        CasRetryFailure::TimedOut {
            scope: RetryTimeoutScope::Attempt,
        }
    ));
    assert_eq!(
        error.failure().timeout_scope(),
        Some(RetryTimeoutScope::Attempt)
    );
    assert_eq!(error.kind(), CasErrorKind::AttemptTimeout);
    assert_eq!(error.error(), None);
}

/// Verifies cancellation preserves its phase and has no business E.
#[cfg(feature = "tokio")]
#[tokio::test]
async fn test_cas_error_maps_cancelled_terminal_without_business_error() {
    let cancellation = RetryCancellationToken::new();
    cancellation.cancel();
    let retry_error = Retry::<CasAttemptFailure<usize, TestError>>::builder(
        terminal_test_policy(1),
    )
    .build()
    .asynchronous()
    .cancellation_token(cancellation)
    .run(|| async { Ok::<_, CasAttemptFailure<usize, TestError>>(()) })
    .await
    .expect_err("pre-attempt cancellation should stop the flow");

    let error = CasError::from(retry_error);
    assert!(matches!(
        error.failure(),
        CasRetryFailure::Cancelled {
            phase: RetryCancellationPhase::BeforeAttempt,
        }
    ));
    assert_eq!(
        error.failure().cancellation_phase(),
        Some(RetryCancellationPhase::BeforeAttempt)
    );
    assert_eq!(error.kind(), CasErrorKind::RetryInfrastructure);
    assert_eq!(error.error(), None);
}

/// Verifies callback attribution is retained instead of becoming business E.
#[test]
fn test_cas_error_maps_callback_failed_terminal_without_business_error() {
    let retry_error = Retry::<CasAttemptFailure<usize, TestError>>::builder(
        terminal_test_policy(1),
    )
    .observer(PanickingStartedObserver)
    .build()
    .sync()
    .run(|| Ok::<_, CasAttemptFailure<usize, TestError>>(()))
    .expect_err("the started observer should fail");

    let error = CasError::from(retry_error);
    let CasRetryFailure::CallbackFailed { callback } = error.failure() else {
        panic!("expected a callback failure");
    };
    assert_eq!(callback.callback(), RetryCallbackKind::Observer);
    assert_eq!(callback.index(), 0);
    assert_eq!(callback.phase(), RetryCallbackPhase::AttemptStarted);
    assert_eq!(error.failure().callback_failure(), Some(callback));
    assert!(error.to_string().contains("observer callback 0"));
    assert_eq!(error.kind(), CasErrorKind::RetryInfrastructure);
    assert_eq!(error.error(), None);
}

/// Verifies timer infrastructure details stay structural even when an earlier
/// business error is retained.
#[test]
fn test_cas_error_maps_infrastructure_terminal_without_reclassification() {
    let timer: Arc<dyn Timer> =
        Arc::new(FaultInjectingTimer::backend_unavailable(
            TimerFailurePoint::Registration,
            "cas-test",
            "offline",
        ));
    let retry_error = Retry::builder(
        RetryPolicy::builder()
            .max_attempts(2)
            .backoff(BackoffPolicy::fixed(Duration::from_millis(1)))
            .build()
            .expect("timer failure policy should be valid"),
    )
    .build()
    .sync()
    .timer(timer)
    .run(|| {
        Err::<(), _>(CasAttemptFailure::Retry {
            current: Arc::new(3),
            error: TestError("retry first"),
        })
    })
    .expect_err("the retry timer should fail");

    let error = CasError::from(retry_error);
    let CasRetryFailure::Infrastructure { failure } = error.failure() else {
        panic!("expected an infrastructure failure");
    };
    assert!(matches!(failure, RetryInfrastructureFailure::Timer { .. }));
    assert_eq!(
        failure.message(),
        Some(
            "monotonic timer is unavailable: timer backend 'cas-test' is unavailable: offline"
        )
    );
    assert_eq!(error.failure().infrastructure_failure(), Some(failure));
    assert!(error.to_string().contains("cas-test"));
    assert_eq!(error.kind(), CasErrorKind::RetryInfrastructure);
    assert_eq!(error.error(), Some(&TestError("retry first")));
}

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
    assert!(matches!(
        error.failure(),
        CasRetryFailure::Exhausted {
            limit: RetryLimitKind::Attempts,
        }
    ));
    assert!(format!("{error:?}").contains("CasError"));
    assert_eq!(
        error.source().map(ToString::to_string),
        Some("still-busy".to_string())
    );
    assert_eq!(error.error(), Some(&TestError("still-busy")));
    assert_eq!(error.current().map(|current| **current), Some(3));
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
    assert_eq!(abort.failure(), &CasRetryFailure::Aborted);
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
    assert!(matches!(
        conflict.failure(),
        CasRetryFailure::Exhausted {
            limit: RetryLimitKind::Attempts,
        }
    ));
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
    assert!(matches!(
        elapsed.failure(),
        CasRetryFailure::Exhausted {
            limit: RetryLimitKind::OperationElapsed,
        }
    ));
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
    assert!(matches!(
        total.failure(),
        CasRetryFailure::Exhausted {
            limit: RetryLimitKind::TotalElapsed,
        }
    ));
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
    assert!(matches!(
        error.failure(),
        CasRetryFailure::TimedOut {
            scope: RetryTimeoutScope::Attempt,
        }
    ));
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
    assert!(matches!(
        exhausted.failure(),
        CasRetryFailure::Exhausted {
            limit: RetryLimitKind::Attempts,
        }
    ));
    assert!(exhausted.to_string().contains("attempt timed out"));
}
