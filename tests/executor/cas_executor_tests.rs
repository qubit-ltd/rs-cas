/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::{CasAttemptFailure, CasContext, CasDecision, CasErrorKind, CasExecutor, CasHooks};

use crate::support::{NonCloneValue, TestError};

/// Verifies sync execution retries CAS conflicts and reports retry hooks.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_execute_retries_conflict_and_calls_retry_hook() {
    let state = AtomicRef::from_value(0usize);
    let attempts = AtomicUsize::new(0);
    let retries = Arc::new(Mutex::new(Vec::new()));
    let retry_events = Arc::clone(&retries);

    let hooks = CasHooks::new().on_retry(
        move |context: &CasContext, failure: &CasAttemptFailure<usize, TestError>| {
            retry_events
                .lock()
                .expect("retry events should be lockable")
                .push((
                    context.attempt(),
                    failure.is_conflict(),
                    **failure.current(),
                ));
        },
    );

    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .no_delay()
        .build()
        .expect("executor should build");

    let success = executor
        .execute_with_hooks(
            &state,
            |current: &usize| {
                if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    state.store(Arc::new(*current + 1));
                }
                CasDecision::update(*current + 1, *current + 10)
            },
            hooks,
        )
        .expect("second attempt should succeed");

    assert!(success.is_updated());
    assert_eq!(
        **success.previous().expect("updated success has previous"),
        1
    );
    assert_eq!(**success.current(), 2);
    assert_eq!(*success.output(), 11);
    assert_eq!(success.attempts(), 2);
    assert_eq!(*state.load(), 2);
    assert_eq!(
        *retries.lock().expect("retry events should be lockable"),
        vec![(1, true, 1)]
    );
}

/// Verifies `finish` returns success without writing a new state.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_execute_finish_returns_without_write() {
    let state = AtomicRef::from_value(9usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .no_delay()
        .build()
        .expect("executor should build");

    let success = executor
        .execute(&state, |_current: &usize| {
            CasDecision::<usize, NonCloneValue, TestError>::finish(NonCloneValue { value: "done" })
        })
        .expect("finish should succeed");

    assert!(!success.is_updated());
    assert_eq!(*success.current().as_ref(), 9);
    assert_eq!(success.output().value, "done");
    assert_eq!(*state.load(), 9);
}

/// Verifies abort decisions stop retrying and trigger abort hooks.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_execute_abort_returns_error_and_calls_abort_hook() {
    let state = AtomicRef::from_value(7usize);
    let aborts = Arc::new(Mutex::new(Vec::new()));
    let abort_events = Arc::clone(&aborts);

    let hooks = CasHooks::new().on_abort(
        move |context: &CasContext, failure: &CasAttemptFailure<usize, TestError>| {
            abort_events
                .lock()
                .expect("abort events should be lockable")
                .push((context.attempt(), failure.error().cloned()));
        },
    );

    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(4)
        .no_delay()
        .build()
        .expect("executor should build");

    let error = executor
        .execute_with_hooks(
            &state,
            |_current: &usize| CasDecision::<usize, (), TestError>::abort(TestError("forbidden")),
            hooks,
        )
        .expect_err("abort should fail");

    assert_eq!(error.kind(), CasErrorKind::Abort);
    assert_eq!(error.attempts(), 1);
    assert_eq!(error.error(), Some(&TestError("forbidden")));
    assert_eq!(error.current().map(|current| **current), Some(7));
    assert_eq!(
        *aborts.lock().expect("abort events should be lockable"),
        vec![(1, Some(TestError("forbidden")))]
    );
}

/// Verifies retryable business failures exhaust attempts with the final error preserved.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_execute_retry_exhausted_preserves_last_error() {
    let state = AtomicRef::from_value(4usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .no_delay()
        .build()
        .expect("executor should build");

    let error = executor
        .execute(&state, |_current: &usize| {
            CasDecision::<usize, (), TestError>::retry(TestError("busy"))
        })
        .expect_err("retry exhaustion should fail");

    assert_eq!(error.kind(), CasErrorKind::RetryExhausted);
    assert_eq!(error.attempts(), 2);
    assert_eq!(error.error(), Some(&TestError("busy")));
    assert!(matches!(error.last_failure(), Some(failure) if failure.is_retry()));
}

/// Verifies max-elapsed exhaustion preserves the last failure.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_execute_max_elapsed_exceeded_preserves_last_failure() {
    let state = AtomicRef::from_value(11usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(10)
        .fixed_delay(Duration::from_millis(5))
        .jitter_factor(0.0)
        .max_elapsed(Some(Duration::from_millis(1)))
        .build()
        .expect("executor should build");

    let error = executor
        .execute(&state, |_current: &usize| {
            CasDecision::<usize, (), TestError>::retry(TestError("again"))
        })
        .expect_err("max elapsed should fail");

    assert_eq!(error.kind(), CasErrorKind::MaxElapsedExceeded);
    assert_eq!(
        error.last_failure().map(|failure| failure.is_retry()),
        Some(true)
    );
    assert_eq!(error.current().map(|current| **current), Some(11));
}

/// Verifies async execution can retry timed-out attempts and then succeed.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[cfg(feature = "tokio")]
#[tokio::test(start_paused = true)]
async fn test_execute_async_retries_timeout_then_succeeds() {
    let state = AtomicRef::from_value(0usize);
    let attempts = Arc::new(AtomicUsize::new(0));
    let retries = Arc::new(Mutex::new(Vec::new()));
    let retry_events = Arc::clone(&retries);

    let hooks = CasHooks::new().on_retry(
        move |context: &CasContext, failure: &CasAttemptFailure<usize, TestError>| {
            retry_events
                .lock()
                .expect("retry events should be lockable")
                .push((context.attempt(), failure.is_timeout()));
        },
    );

    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .no_delay()
        .attempt_timeout(Some(Duration::from_millis(10)))
        .retry_on_timeout()
        .build()
        .expect("executor should build");

    let success = executor
        .execute_async_with_hooks(
            &state,
            {
                let attempts = Arc::clone(&attempts);
                move |current: Arc<usize>| {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                    async move {
                        if attempt == 0 {
                            tokio::time::sleep(Duration::from_millis(20)).await;
                            CasDecision::<usize, usize, TestError>::finish(999)
                        } else {
                            CasDecision::update(*current + 1, *current + 100)
                        }
                    }
                }
            },
            hooks,
        )
        .await
        .expect("second async attempt should succeed");

    assert_eq!(success.attempts(), 2);
    assert_eq!(
        success.context().attempt_timeout(),
        Some(Duration::from_millis(10))
    );
    assert_eq!(**success.current(), 1);
    assert_eq!(*success.output(), 100);
    assert_eq!(
        *retries.lock().expect("retry events should be lockable"),
        vec![(1, true)]
    );
}

/// Verifies async timeout abort policy surfaces `AttemptTimeout`.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[cfg(feature = "tokio")]
#[tokio::test(start_paused = true)]
async fn test_execute_async_timeout_abort_returns_attempt_timeout() {
    let state = AtomicRef::from_value(5usize);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .no_delay()
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
        .expect_err("timeout should abort");

    assert_eq!(error.kind(), CasErrorKind::AttemptTimeout);
    assert_eq!(error.attempts(), 1);
    assert_eq!(error.current().map(|current| **current), Some(5));
}
