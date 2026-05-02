/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use qubit_atomic::AtomicRef;
use qubit_cas::{
    CasAttemptFailureKind, CasDecision, CasErrorKind, CasEvent, CasExecutionOutcome, CasExecutor,
    CasHooks, CasObservabilityConfig, ContentionThresholds, ListenerPanicPolicy,
};

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

    let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
        if let CasEvent::AttemptFailed { context, kind } = event {
            retry_events
                .lock()
                .expect("retry events should be lockable")
                .push((context.attempt(), *kind));
        }
    });

    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .no_delay()
        .observability(CasObservabilityConfig::event_stream())
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
        vec![(1, CasAttemptFailureKind::Conflict)]
    );
}

/// Verifies execution reports expose conflict counts and ratios.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_execute_returns_report_with_conflict_counts() {
    let state = AtomicRef::from_value(0usize);
    let attempts = AtomicUsize::new(0);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .no_delay()
        .build()
        .expect("executor should build");

    let outcome = executor.execute(&state, |current: &usize| {
        if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
            state.store(Arc::new(*current + 1));
        }
        CasDecision::update(*current + 1, *current + 10)
    });
    let report = outcome.report().clone();
    let success = outcome.expect("second attempt should succeed");

    assert_eq!(success.attempts(), 2);
    assert_eq!(report.attempts_total(), 2);
    assert_eq!(report.conflicts(), 1);
    assert_eq!(report.outcome(), CasExecutionOutcome::SuccessUpdated);
    assert_eq!(report.conflict_ratio(), 0.5);
}

/// Verifies contention alerts are emitted when report thresholds are crossed.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_execute_emits_contention_alert() {
    let state = AtomicRef::from_value(0usize);
    let attempts = AtomicUsize::new(0);
    let alerts = Arc::new(Mutex::new(Vec::new()));
    let alert_events = Arc::clone(&alerts);
    let thresholds = ContentionThresholds::new(2, 1, 0.5);
    let hooks = CasHooks::new().on_alert(move |alert: &qubit_cas::CasAlert| {
        alert_events
            .lock()
            .expect("alert events should be lockable")
            .push((
                alert.report().attempts_total(),
                alert.report().conflicts(),
                alert.thresholds(),
            ));
    });
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .no_delay()
        .observability(CasObservabilityConfig::event_stream_with_alert(thresholds))
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

    assert_eq!(success.attempts(), 2);
    assert_eq!(
        *alerts.lock().expect("alert events should be lockable"),
        vec![(2, 1, thresholds)]
    );
}

/// Verifies event listener panics can be isolated.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_execute_isolates_event_listener_panics() {
    let state = AtomicRef::from_value(3usize);
    let hooks = CasHooks::new().on_event(|_event: &CasEvent| {
        panic!("event listener panic should be isolated");
    });
    let executor = CasExecutor::<usize, TestError>::builder()
        .no_delay()
        .observability(CasObservabilityConfig::event_stream())
        .isolate_listener_panics()
        .build()
        .expect("executor should build");

    let success = executor
        .execute_with_hooks(
            &state,
            |_current: &usize| CasDecision::<usize, &'static str, TestError>::finish("ok"),
            hooks,
        )
        .expect("listener panic should be isolated");

    assert_eq!(*success.output(), "ok");
}

/// Verifies alert listener panics can be isolated.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_execute_isolates_alert_listener_panics() {
    let state = AtomicRef::from_value(0usize);
    let attempts = AtomicUsize::new(0);
    let thresholds = ContentionThresholds::new(2, 1, 0.5);
    let hooks = CasHooks::new().on_alert(|_alert: &qubit_cas::CasAlert| {
        panic!("alert listener panic should be isolated");
    });
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .no_delay()
        .observability(
            CasObservabilityConfig::event_stream_with_alert(thresholds)
                .with_listener_panic_policy(ListenerPanicPolicy::Isolate),
        )
        .build()
        .expect("executor should build");

    let success = executor
        .execute_with_hooks(
            &state,
            |current: &usize| {
                if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    state.store(Arc::new(*current + 1));
                }
                CasDecision::update(*current + 1, ())
            },
            hooks,
        )
        .expect("alert listener panic should be isolated");

    assert_eq!(success.attempts(), 2);
}

/// Verifies alert mode does not require an alert listener.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[test]
fn test_execute_alert_mode_without_alert_hook_succeeds() {
    let state = AtomicRef::from_value(0usize);
    let attempts = AtomicUsize::new(0);
    let thresholds = ContentionThresholds::new(2, 1, 0.5);
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .no_delay()
        .observability(CasObservabilityConfig::event_stream_with_alert(thresholds))
        .build()
        .expect("executor should build");

    let success = executor
        .execute(&state, |current: &usize| {
            if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                state.store(Arc::new(*current + 1));
            }
            CasDecision::update(*current + 1, ())
        })
        .expect("alert mode should not require an alert hook");

    assert_eq!(success.attempts(), 2);
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

    let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
        if let CasEvent::AttemptFailed { context, kind } = event {
            abort_events
                .lock()
                .expect("abort events should be lockable")
                .push((context.attempt(), *kind));
        }
    });

    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(4)
        .no_delay()
        .observability(CasObservabilityConfig::event_stream())
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
        vec![(1, CasAttemptFailureKind::Abort)]
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
        .no_delay()
        .max_operation_elapsed(Some(Duration::from_millis(1)))
        .build()
        .expect("executor should build");

    let error = executor
        .execute(&state, |_current: &usize| {
            std::thread::sleep(Duration::from_millis(2));
            CasDecision::<usize, (), TestError>::retry(TestError("again"))
        })
        .expect_err("max elapsed should fail");

    assert_eq!(error.kind(), CasErrorKind::MaxOperationElapsedExceeded);
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

    let hooks = CasHooks::new().on_event(move |event: &CasEvent| {
        if let CasEvent::AttemptFailed { context, kind } = event {
            retry_events
                .lock()
                .expect("retry events should be lockable")
                .push((context.attempt(), *kind));
        }
    });

    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(3)
        .no_delay()
        .attempt_timeout(Some(Duration::from_millis(10)))
        .retry_on_timeout()
        .observability(CasObservabilityConfig::event_stream())
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
        vec![(1, CasAttemptFailureKind::Timeout)]
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

/// Verifies async execution covers all CAS decision variants.
///
/// # Parameters
/// This test has no parameters.
///
/// # Returns
/// This test returns nothing.
#[cfg(feature = "tokio")]
#[tokio::test]
async fn test_execute_async_covers_decision_variants() {
    let executor = CasExecutor::<usize, TestError>::builder()
        .max_attempts(2)
        .no_delay()
        .build()
        .expect("executor should build");

    let finish_state = AtomicRef::from_value(1usize);
    let finish = executor
        .execute_async(&finish_state, |_current: Arc<usize>| async move {
            CasDecision::<usize, &'static str, TestError>::finish("done")
        })
        .await
        .expect("async finish should succeed");
    assert_eq!(*finish.output(), "done");

    let retry_state = AtomicRef::from_value(2usize);
    let retry = executor
        .execute_async(&retry_state, |_current: Arc<usize>| async move {
            CasDecision::<usize, (), TestError>::retry(TestError("retry"))
        })
        .await
        .expect_err("async retry should exhaust attempts");
    assert_eq!(retry.kind(), CasErrorKind::RetryExhausted);

    let abort_state = AtomicRef::from_value(3usize);
    let abort = executor
        .execute_async(&abort_state, |_current: Arc<usize>| async move {
            CasDecision::<usize, (), TestError>::abort(TestError("abort"))
        })
        .await
        .expect_err("async abort should fail");
    assert_eq!(abort.kind(), CasErrorKind::Abort);

    let conflict_state = AtomicRef::from_value(4usize);
    let conflict = executor
        .execute_async(&conflict_state, |current: Arc<usize>| {
            conflict_state.store(Arc::new(*current + 1));
            async move { CasDecision::<usize, (), TestError>::update(*current + 2, ()) }
        })
        .await
        .expect_err("async conflict should exhaust attempts");
    assert_eq!(conflict.kind(), CasErrorKind::Conflict);
}
