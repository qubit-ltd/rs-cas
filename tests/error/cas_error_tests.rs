/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::{CasDecision, CasErrorKind, CasExecutor};
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
    assert_eq!(error.reason(), RetryErrorReason::AttemptsExceeded);
    assert!(format!("{error:?}").contains("CasError"));
    assert_eq!(
        error.source().map(ToString::to_string),
        Some("still-busy".to_string())
    );
    assert_eq!(error.error(), Some(&TestError("still-busy")));
    assert_eq!(error.current().map(|current| **current), Some(3));
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
    assert_eq!(conflict.reason(), RetryErrorReason::AttemptsExceeded);
    assert!(conflict.to_string().contains("conflicts exhausted"));
    assert_eq!(conflicts.load(Ordering::SeqCst), 2);

    let elapsed_state = AtomicRef::from_value(12usize);
    let elapsed_executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .no_delay()
        .max_elapsed(Some(Duration::from_millis(1)))
        .build()
        .expect("executor should build");
    let elapsed = elapsed_executor
        .execute(&elapsed_state, |_current: &usize| {
            std::thread::sleep(Duration::from_millis(2));
            CasDecision::<usize, (), TestError>::retry(TestError("slow"))
        })
        .expect_err("elapsed budget should fail");
    assert_eq!(elapsed.kind(), CasErrorKind::MaxElapsedExceeded);
    assert_eq!(elapsed.reason(), RetryErrorReason::MaxElapsedExceeded);
    assert!(elapsed.to_string().contains("max elapsed exceeded"));
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
    assert_eq!(error.reason(), RetryErrorReason::Aborted);
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
    assert_eq!(exhausted.reason(), RetryErrorReason::AttemptsExceeded);
    assert!(exhausted.to_string().contains("attempt timed out"));
}
